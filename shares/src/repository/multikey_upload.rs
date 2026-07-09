//! `POST /api/storage/upload-multikey`, `evict-from-folder`, and
//! `move-into-shared`.
//!
//! All three share two cross-cutting checks:
//! - the caller's snapshot of the destination's membership is fresh
//!   enough (TOCTOU defense), and
//! - the per-member key set covers exactly the current member set.
//!
//! Failing either path returns 409 with the current member roster so the
//! client can refresh and retry.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use chrono::Utc;
use cryptfns::asn1::{
    encode_audit_event_sig_input_v1, AuditEventActionEnum, AuditEventSigInputV1,
};
use cryptfns::identity::KeyType;
use entity::{
    file_tokens, files,
    permission::{permission, SharePermission},
    tokens, user_files, users, ActiveValue, ColumnTrait, EntityTrait, QueryFilter,
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};

use crate::{
    contracts::audit::NewAuditEvent,
    data::{
        folder_members::FolderMembersResponse,
        multikey_upload::{
            EvictFromFolderBody, MemberKey, MembersListSnapshot, MoveIntoSharedBody,
            UploadMultikeyBody,
        },
    },
    repository::{audit, move_subtree, queries, Repository},
};
use validr::Validation;

const REPLAY_WINDOW_SECONDS: i64 = 300;

pub(crate) struct MultikeyUploadOutput {
    pub file_id: Uuid,
}

pub(crate) struct MoveOutput {
    pub file_id: Uuid,
}

pub(crate) struct EvictOutput {
    pub file_id: Uuid,
}

/// 409 `share_membership_changed` body — clients refresh their member
/// roster from the embedded list, re-verify fingerprints, re-wrap, and
/// retry.
fn share_membership_changed(current_members: FolderMembersResponse) -> Error {
    Error::Conflict(
        serde_json::to_string(&serde_json::json!({
            "code": "share_membership_changed",
            "current_members": current_members,
        }))
        .unwrap_or_else(|_| "share_membership_changed".to_string()),
    )
}

impl Repository<'_> {
    /// Multi-key upload into a shared folder. Caller is the uploader (B),
    /// owns the resulting file, and supplies one wrapped key per current
    /// member of the destination folder.
    pub(crate) async fn upload_multikey(
        &self,
        caller: &users::Model,
        body: UploadMultikeyBody,
    ) -> AppResult<MultikeyUploadOutput> {
        let body = body.validate()?;
        let new_file_id = Uuid::parse_str(&body.new_file_id.clone().unwrap())
            .map_err(|_| Error::BadRequest("new_file_id_invalid".to_string()))?;
        let parent_id = Uuid::parse_str(&body.parent_file_id.clone().unwrap())
            .map_err(|_| Error::BadRequest("parent_file_id_invalid".to_string()))?;
        let signed_timestamp = body.timestamp.unwrap();
        let event_signature = body.event_signature.clone().unwrap();
        let snapshot = body
            .members_list_snapshot
            .clone()
            .ok_or_else(|| Error::BadRequest("members_list_snapshot_required".to_string()))?;
        let raw_keys = body
            .member_keys
            .clone()
            .ok_or_else(|| Error::BadRequest("member_keys_required".to_string()))?;
        if raw_keys.is_empty() {
            return Err(Error::BadRequest("member_keys_empty".to_string()));
        }

        replay_window(signed_timestamp)?;

        let perm = permission(&self.context.db, parent_id, caller.id).await?;
        if matches!(perm, SharePermission::None) {
            return Err(Error::NotFound("folder_not_found".to_string()));
        }
        if !perm.can_write() {
            return Err(Error::Forbidden("forbidden_not_writer".to_string()));
        }

        let folder = files::Entity::find_by_id(parent_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("folder_not_found".to_string()))?;
        if folder.mime != "dir" {
            return Err(Error::BadRequest("not_a_folder".to_string()));
        }

        let member_keys = parse_member_keys(&raw_keys)?;
        let current_member_rows = current_member_rows(self, parent_id).await?;
        verify_member_keys_match(
            self,
            parent_id,
            &member_keys,
            &current_member_rows,
        )
        .await?;
        verify_snapshot_freshness(self, parent_id, &folder, &snapshot).await?;
        verify_ownership_claim(caller.id, &member_keys)?;

        // Verify the audit-event signature server-side. The client
        // supplied `new_file_id` so the signed payload binds to a
        // stable id we can match — collisions on the unique PK 409 at
        // insert time, defending against a malicious caller reusing an
        // existing file id.
        let sig_input = AuditEventSigInputV1 {
            sender_id: caller.id.into_bytes(),
            recipient_id: None,
            file_id: new_file_id.into_bytes(),
            action: AuditEventActionEnum::SharedFolderUpload,
            share_role_before: None,
            share_role_after: None,
            timestamp: signed_timestamp,
        };
        verify_event_signature(&sig_input, &event_signature, caller)?;

        // Quota check uses owner-only storage.
        let claimed_size = body.size.unwrap_or(0);
        if let Some(quota) = caller.quota {
            let used: i64 = owner_used_space(self, caller.id).await?;
            if used + claimed_size > quota {
                return Err(Error::BadRequest("quota_exceeded".to_string()));
            }
        }

        // Build a lookup from user_id → row so we can read each
        // member's inherited role and σ_member signature for the new
        // file in one pass below.
        let by_user: HashMap<Uuid, user_files::Model> = current_member_rows
            .iter()
            .cloned()
            .map(|r| (r.user_id, r))
            .collect();

        let now = Utc::now().timestamp();
        let file_active = build_new_file(&body, new_file_id, parent_id, now);

        let tx = self.context.db.begin().await?;
        files::Entity::insert(file_active)
            .exec_without_returning(&tx)
            .await?;

        for entry in &member_keys {
            let inherited = by_user
                .get(&entry.user_id)
                .map(|r| r.share_role.clone())
                .unwrap_or_else(|| "reader".to_string());
            // For non-owner rows, σ_member is inherited from the
            // folder share's per-recipient row — that signature already
            // proves the recipient's pubkey is anointed by the folder
            // owner or a current Co-owner.
            let inherited_member_signature = by_user
                .get(&entry.user_id)
                .and_then(|r| r.member_signature.clone());
            let row = user_files::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                file_id: ActiveValue::Set(new_file_id),
                user_id: ActiveValue::Set(entry.user_id),
                encrypted_key: ActiveValue::Set(entry.encrypted_key.clone()),
                is_owner: ActiveValue::Set(entry.is_owner_of_file),
                created_at: ActiveValue::Set(now),
                expires_at: ActiveValue::Set(None),
                share_role: ActiveValue::Set(inherited),
                shared_at: ActiveValue::Set(if entry.is_owner_of_file {
                    None
                } else {
                    Some(signed_timestamp)
                }),
                shared_by_user_id: ActiveValue::Set(if entry.is_owner_of_file {
                    None
                } else {
                    Some(caller.id)
                }),
                member_signature: ActiveValue::Set(if entry.is_owner_of_file {
                    None
                } else {
                    inherited_member_signature
                }),
            };
            user_files::Entity::insert(row)
                .exec_without_returning(&tx)
                .await?;
        }

        if let Some(token_hashes) = body.search_tokens_hashed.clone() {
            if !token_hashes.is_empty() {
                upsert_tokens(&tx, new_file_id, token_hashes).await?;
            }
        }

        audit::append_event(
            &tx,
            NewAuditEvent {
                sender_id: Some(caller.id),
                recipient_id: None,
                file_id: new_file_id,
                action_str: "shared_folder_upload",
                share_role_before: None,
                share_role_after: None,
                created_at: signed_timestamp,
                event_signature: Some(event_signature.clone()),
            },
        )
        .await?;

        tx.commit().await?;

        Ok(MultikeyUploadOutput {
            file_id: new_file_id,
        })
    }

    /// Owner-only — evict a contributor's file from the folder it was
    /// uploaded into. Sets the file's `file_id` (parent pointer) to NULL
    /// and drops every non-owner `user_files` row, so the contribution
    /// reverts to a private file in the contributor's drive root.
    pub(crate) async fn evict_from_folder(
        &self,
        caller: &users::Model,
        target_file_id: Uuid,
        body: EvictFromFolderBody,
    ) -> AppResult<EvictOutput> {
        let body = body.validate()?;
        let signed_timestamp = body.timestamp.unwrap();
        let event_signature = body.event_signature.clone().unwrap();

        replay_window(signed_timestamp)?;

        let file = files::Entity::find_by_id(target_file_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;
        let parent_id = file
            .file_id
            .ok_or_else(|| Error::BadRequest("file_not_in_folder".to_string()))?;

        let perm = permission(&self.context.db, parent_id, caller.id).await?;
        if !matches!(perm, SharePermission::Owner) {
            return Err(Error::Forbidden("forbidden_not_owner".to_string()));
        }

        let owner_row = user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(target_file_id))
            .filter(user_files::Column::IsOwner.eq(true))
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::InternalError("file_has_no_owner_row".to_string()))?;
        if owner_row.user_id == caller.id {
            // The folder owner *is* the file owner — that's a regular
            // delete or move-out, not an eviction.
            return Err(Error::BadRequest("cannot_evict_own_file".to_string()));
        }

        let sig_input = AuditEventSigInputV1 {
            sender_id: caller.id.into_bytes(),
            recipient_id: Some(owner_row.user_id.into_bytes()),
            file_id: target_file_id.into_bytes(),
            action: AuditEventActionEnum::SharedFolderEvict,
            share_role_before: None,
            share_role_after: None,
            timestamp: signed_timestamp,
        };
        verify_event_signature(&sig_input, &event_signature, caller)?;

        let tx = self.context.db.begin().await?;

        files::Entity::update(files::ActiveModel {
            id: ActiveValue::Unchanged(target_file_id),
            file_id: ActiveValue::Set(None),
            ..Default::default()
        })
        .exec(&tx)
        .await?;

        user_files::Entity::delete_many()
            .filter(user_files::Column::FileId.eq(target_file_id))
            .filter(user_files::Column::IsOwner.eq(false))
            .exec(&tx)
            .await?;

        audit::append_event(
            &tx,
            NewAuditEvent {
                sender_id: Some(caller.id),
                recipient_id: Some(owner_row.user_id),
                file_id: target_file_id,
                action_str: "shared_folder_evict",
                share_role_before: None,
                share_role_after: None,
                created_at: signed_timestamp,
                event_signature: Some(event_signature.clone()),
            },
        )
        .await?;

        tx.commit().await?;

        Ok(EvictOutput {
            file_id: target_file_id,
        })
    }

    /// Caller relocates a file or folder they own into a shared folder,
    /// re-wrapping the moved file key(s) for every current member of the
    /// destination and re-binding the per-file `user_files` set to the
    /// destination's roster.
    ///
    /// A single file carries flat `member_keys`; a folder carries
    /// `entries` covering the whole subtree. Both shapes share the
    /// destination auth, roster freshness, and signature checks below;
    /// the per-node row rebind and the choice of single-vs-cascade is
    /// delegated to [`move_subtree`].
    pub(crate) async fn move_into_shared(
        &self,
        caller: &users::Model,
        body: MoveIntoSharedBody,
    ) -> AppResult<MoveOutput> {
        let body = body.validate()?;
        let file_id = Uuid::parse_str(&body.file_id.clone().unwrap())
            .map_err(|_| Error::BadRequest("file_id_invalid".to_string()))?;
        let dest_id = Uuid::parse_str(&body.destination_folder_id.clone().unwrap())
            .map_err(|_| Error::BadRequest("destination_folder_id_invalid".to_string()))?;
        let signed_timestamp = body.timestamp.unwrap();
        let event_signature = body.event_signature.clone().unwrap();
        let snapshot = body
            .members_list_snapshot
            .clone()
            .ok_or_else(|| Error::BadRequest("members_list_snapshot_required".to_string()))?;

        replay_window(signed_timestamp)?;

        let file_perm = permission(&self.context.db, file_id, caller.id).await?;
        if matches!(file_perm, SharePermission::None) {
            return Err(Error::NotFound("file_not_found".to_string()));
        }
        if !file_perm.can_write() {
            return Err(Error::Forbidden("forbidden_not_writer".to_string()));
        }

        let dest_perm = permission(&self.context.db, dest_id, caller.id).await?;
        if matches!(dest_perm, SharePermission::None) {
            return Err(Error::NotFound("destination_folder_not_found".to_string()));
        }
        if !dest_perm.can_write() {
            return Err(Error::Forbidden(
                "forbidden_not_writer_on_destination".to_string(),
            ));
        }

        let dest = files::Entity::find_by_id(dest_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("destination_folder_not_found".to_string()))?;
        if dest.mime != "dir" {
            return Err(Error::BadRequest("destination_not_a_folder".to_string()));
        }

        let source = files::Entity::find_by_id(file_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

        let owner_row = user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(file_id))
            .filter(user_files::Column::IsOwner.eq(true))
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::InternalError("file_has_no_owner_row".to_string()))?;

        // Only the file's owner may relocate it into a shared folder. A
        // non-owner's only "move out of my view"
        // semantic is self-remove, never a re-parent of someone else's
        // file.
        if owner_row.user_id != caller.id {
            return Err(Error::Forbidden("cannot_move_not_owner".to_string()));
        }

        // Re-parenting a folder into itself or one of its own descendants
        // would create a parent cycle, and the recursive subtree walks
        // (`queries::file_tree_ids`) that every later revoke/move/list runs
        // would then loop without bound. `file_id` is always a member of its
        // own subtree, so this rejects `dest_id == file_id` too. The plain
        // move route guards the same way with `cannot_move_to_itself`.
        let source_subtree = queries::file_tree_ids(&self.context.db, file_id).await?;
        if source_subtree.contains(&dest_id) {
            return Err(Error::BadRequest("cannot_move_into_own_subtree".to_string()));
        }

        let current_member_rows = current_member_rows(self, dest_id).await?;
        verify_snapshot_freshness(self, dest_id, &dest, &snapshot).await?;

        // The audit canonical binds the moved ROOT id regardless of shape,
        // so a folder cascade and a single-file move sign the same way.
        let sig_input = AuditEventSigInputV1 {
            sender_id: caller.id.into_bytes(),
            recipient_id: None,
            file_id: file_id.into_bytes(),
            action: AuditEventActionEnum::SharedFolderUpload,
            share_role_before: None,
            share_role_after: None,
            timestamp: signed_timestamp,
        };
        verify_event_signature(&sig_input, &event_signature, caller)?;

        match body.entries.clone() {
            Some(raw_entries) => {
                let root_id = move_subtree::move_folder_into_shared(
                    self,
                    caller,
                    file_id,
                    dest_id,
                    raw_entries,
                    &current_member_rows,
                    signed_timestamp,
                    &event_signature,
                )
                .await?;
                Ok(MoveOutput { file_id: root_id })
            }
            None => {
                // Flat single-file shape. A folder here would re-parent
                // only its own node and leave every descendant encrypted
                // for the original owner — refuse it (S1).
                if source.mime == "dir" {
                    return Err(Error::BadRequest(
                        "move_folder_requires_cascade".to_string(),
                    ));
                }
                let raw_keys = body
                    .member_keys
                    .clone()
                    .ok_or_else(|| Error::BadRequest("member_keys_required".to_string()))?;
                if raw_keys.is_empty() {
                    return Err(Error::BadRequest("member_keys_empty".to_string()));
                }
                let member_keys = parse_member_keys(&raw_keys)?;
                verify_member_keys_match(self, dest_id, &member_keys, &current_member_rows)
                    .await?;

                let tx = self.context.db.begin().await?;
                files::Entity::update(files::ActiveModel {
                    id: ActiveValue::Unchanged(file_id),
                    file_id: ActiveValue::Set(Some(dest_id)),
                    ..Default::default()
                })
                .exec(&tx)
                .await?;
                move_subtree::rebind_node_to_roster(
                    &tx,
                    file_id,
                    &member_keys,
                    current_member_rows,
                    caller.id,
                    signed_timestamp,
                )
                .await?;
                audit::append_event(
                    &tx,
                    NewAuditEvent {
                        sender_id: Some(caller.id),
                        recipient_id: None,
                        file_id,
                        action_str: "shared_folder_upload",
                        share_role_before: None,
                        share_role_after: None,
                        created_at: signed_timestamp,
                        event_signature: Some(event_signature.clone()),
                    },
                )
                .await?;
                tx.commit().await?;
                Ok(MoveOutput { file_id })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ParsedMemberKey {
    pub(super) user_id: Uuid,
    pub(super) encrypted_key: String,
    pub(super) is_owner_of_file: bool,
}

pub(super) fn parse_member_keys(raw: &[MemberKey]) -> AppResult<Vec<ParsedMemberKey>> {
    let mut out = Vec::with_capacity(raw.len());
    let mut seen = HashSet::with_capacity(raw.len());
    for k in raw {
        let validated = k.clone().validate()?;
        let user_id_str = validated.user_id.unwrap();
        let encrypted_key = validated.encrypted_key.unwrap();
        let user_id = Uuid::from_str(&user_id_str)
            .map_err(|_| Error::BadRequest("member_key_user_id_invalid".to_string()))?;
        if !seen.insert(user_id) {
            return Err(Error::BadRequest("member_key_duplicate".to_string()));
        }
        if encrypted_key.is_empty() {
            return Err(Error::BadRequest("member_key_encrypted_key_empty".to_string()));
        }
        out.push(ParsedMemberKey {
            user_id,
            encrypted_key,
            is_owner_of_file: validated.is_owner_of_file.unwrap_or(false),
        });
    }
    Ok(out)
}

pub(super) fn replay_window(timestamp: i64) -> AppResult<()> {
    let now = Utc::now().timestamp();
    if (now - timestamp).abs() > REPLAY_WINDOW_SECONDS {
        return Err(Error::BadRequest("replay_timestamp_skew".to_string()));
    }
    Ok(())
}

pub(super) fn verify_event_signature(
    sig_input: &AuditEventSigInputV1,
    signature_b64: &str,
    signer: &users::Model,
) -> AppResult<()> {
    let der = encode_audit_event_sig_input_v1(sig_input)
        .map_err(|e| Error::CryptoError(Box::new(e)))?;
    let mut signing_input =
        Vec::with_capacity(cryptfns::asn1::AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(cryptfns::asn1::AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    KeyType::from_str(&signer.key_type)?
        .verify_bytes(&signing_input, signature_b64, &signer.pubkey)
        .map_err(|_| Error::BadRequest("event_signature_invalid".to_string()))?;
    Ok(())
}

fn verify_ownership_claim(caller_id: Uuid, keys: &[ParsedMemberKey]) -> AppResult<()> {
    let owners: Vec<&ParsedMemberKey> = keys.iter().filter(|k| k.is_owner_of_file).collect();
    if owners.len() != 1 {
        return Err(Error::BadRequest("invalid_ownership_claim".to_string()));
    }
    if owners[0].user_id != caller_id {
        return Err(Error::BadRequest("invalid_ownership_claim".to_string()));
    }
    Ok(())
}

pub(super) async fn current_member_rows(
    repo: &Repository<'_>,
    folder_id: Uuid,
) -> AppResult<Vec<user_files::Model>> {
    Ok(user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder_id))
        .all(&repo.context.db)
        .await?)
}

pub(super) async fn verify_member_keys_match(
    repo: &Repository<'_>,
    folder_id: Uuid,
    keys: &[ParsedMemberKey],
    current_rows: &[user_files::Model],
) -> AppResult<()> {
    let supplied: HashSet<Uuid> = keys.iter().map(|k| k.user_id).collect();
    let current: HashSet<Uuid> = current_rows.iter().map(|r| r.user_id).collect();
    if supplied != current {
        let current_view = repo
            .folder_members(
                // synthesise a "system-internal" caller from the folder
                // owner so the helper passes its own permission check.
                &owner_user_for_membership_view(repo, folder_id).await?,
                folder_id,
            )
            .await?;
        return Err(share_membership_changed(current_view));
    }
    Ok(())
}

async fn owner_user_for_membership_view(
    repo: &Repository<'_>,
    folder_id: Uuid,
) -> AppResult<users::Model> {
    let owner_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .one(&repo.context.db)
        .await?
        .ok_or_else(|| Error::InternalError("folder_has_no_owner_row".to_string()))?;
    users::Entity::find_by_id(owner_row.user_id)
        .one(&repo.context.db)
        .await?
        .ok_or_else(|| Error::InternalError("folder_owner_missing".to_string()))
}

async fn verify_snapshot_freshness(
    repo: &Repository<'_>,
    folder_id: Uuid,
    folder: &files::Model,
    snapshot: &MembersListSnapshot,
) -> AppResult<()> {
    let signed_at = snapshot.members_signed_at.unwrap_or(0);
    let last_change = folder.last_membership_change_at.unwrap_or(0);
    if signed_at < last_change {
        let current_view = repo
            .folder_members(
                &owner_user_for_membership_view(repo, folder_id).await?,
                folder_id,
            )
            .await?;
        return Err(share_membership_changed(current_view));
    }
    Ok(())
}

pub(crate) async fn owner_used_space(repo: &Repository<'_>, user_id: Uuid) -> AppResult<i64> {
    // Sum of files.size for owner rows. Cheaper than reaching back into
    // the storage repository for one number; matches the same predicate
    // (`is_owner=true`) the existing `Query::used_space` uses.
    let rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .all(&repo.context.db)
        .await?;
    if rows.is_empty() {
        return Ok(0);
    }
    let file_ids: Vec<Uuid> = rows.iter().map(|r| r.file_id).collect();
    let files: Vec<files::Model> = files::Entity::find()
        .filter(files::Column::Id.is_in(file_ids))
        .all(&repo.context.db)
        .await?;
    Ok(files.iter().filter_map(|f| f.size).sum())
}

fn build_new_file(
    body: &UploadMultikeyBody,
    new_file_id: Uuid,
    parent_id: Uuid,
    now: i64,
) -> files::ActiveModel {
    files::ActiveModel {
        id: ActiveValue::Set(new_file_id),
        name_hash: ActiveValue::Set(body.name_hash.clone().unwrap_or_default()),
        encrypted_name: ActiveValue::Set(body.encrypted_name.clone().unwrap_or_default()),
        encrypted_thumbnail: ActiveValue::Set(body.encrypted_thumbnail.clone()),
        mime: ActiveValue::Set(body.mime.clone().unwrap_or_else(|| "text/plain".to_string())),
        size: ActiveValue::Set(body.size),
        chunks: ActiveValue::Set(body.chunks),
        chunks_stored: ActiveValue::Set(Some(0)),
        file_id: ActiveValue::Set(Some(parent_id)),
        md5: ActiveValue::Set(body.md5.clone()),
        sha1: ActiveValue::Set(body.sha1.clone()),
        sha256: ActiveValue::Set(body.sha256.clone()),
        blake2b: ActiveValue::Set(body.blake2b.clone()),
        cipher: ActiveValue::Set(body.cipher.clone().unwrap_or_else(|| "ascon128a".to_string())),
        editable: ActiveValue::Set(body.editable.unwrap_or(false)),
        file_modified_at: ActiveValue::Set(parse_modified_at(body.file_modified_at.as_deref(), now)),
        created_at: ActiveValue::Set(now),
        finished_upload_at: ActiveValue::Set(None),
        active_version: ActiveValue::Set(1),
        pending_version: ActiveValue::Set(None),
        pending_chunks: ActiveValue::Set(None),
        pending_size: ActiveValue::Set(None),
        last_membership_change_at: ActiveValue::Set(None),
        members_list_signature: ActiveValue::Set(None),
        members_list_signed_at: ActiveValue::Set(None),
        members_list_signed_by_user_id: ActiveValue::Set(None),
    }
}

fn parse_modified_at(input: Option<&str>, fallback: i64) -> i64 {
    input
        .and_then(|s| util::datetime::parse_into_naive_datetime(s, Some("file_modified_at")).ok())
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(fallback)
}

pub(crate) async fn upsert_tokens<C: entity::ConnectionTrait>(
    tx: &C,
    file_id: Uuid,
    hashed_tokens: Vec<String>,
) -> AppResult<()> {
    let parsed = cryptfns::tokenizer::from_vec(hashed_tokens)?;
    if parsed.is_empty() {
        return Ok(());
    }

    let existing = tokens::Entity::find()
        .filter(
            tokens::Column::Hash.is_in(
                parsed
                    .iter()
                    .map(|t| t.token.clone())
                    .collect::<Vec<String>>(),
            ),
        )
        .all(tx)
        .await?;

    let mut new_tokens: Vec<tokens::ActiveModel> = Vec::new();
    let mut links: Vec<file_tokens::ActiveModel> = Vec::new();

    for token in parsed {
        let token_id = if let Some(existing) = existing.iter().find(|t| t.hash == token.token) {
            existing.id
        } else {
            let id = Uuid::new_v4();
            new_tokens.push(tokens::ActiveModel {
                id: ActiveValue::Set(id),
                hash: ActiveValue::Set(token.token.clone()),
            });
            id
        };
        links.push(file_tokens::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            file_id: ActiveValue::Set(file_id),
            token_id: ActiveValue::Set(token_id),
            weight: ActiveValue::Set(token.weight as i32),
        });
    }

    if !new_tokens.is_empty() {
        tokens::Entity::insert_many(new_tokens)
            .exec_without_returning(tx)
            .await?;
    }
    if !links.is_empty() {
        file_tokens::Entity::insert_many(links)
            .exec_without_returning(tx)
            .await?;
    }
    Ok(())
}
