//! Subtree-aware relocation of files between private and shared scopes:
//!
//! * `move_folder_into_shared` — cascade variant of
//!   `POST /api/storage/move-into-shared`: re-parent a folder and re-bind
//!   every descendant's `user_files` set to the destination roster.
//! * `move_out_of_shared` — `POST /api/storage/move-out-of-shared`: the
//!   file owner detaches their own file (or folder subtree) from a shared
//!   folder, dropping every other member's access.
//!
//! Both recompute the moved subtree from the server's own `files` table
//! (`queries::file_tree_ids`) and never trust the client's notion of the
//! tree shape — the client only supplies per-node key wraps.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use chrono::Utc;
use cryptfns::asn1::{AuditEventActionEnum, AuditEventSigInputV1};
use entity::{
    files, user_files, users, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};

use crate::{
    contracts::audit::NewAuditEvent,
    data::multikey_upload::{CascadeEntry, MoveOutOfSharedBody},
    repository::{
        audit,
        multikey_upload::{
            parse_member_keys, replay_window, verify_event_signature, verify_member_keys_match,
            ParsedMemberKey,
        },
        queries, Repository,
    },
};
use validr::Validation;

/// Server-side cap on how many files one move request may touch, matching
/// `create_share`'s `ENTRIES_PER_REQUEST_CAP`.
const ENTRIES_PER_REQUEST_CAP: usize = 5000;

/// Re-bind a single file's non-owner `user_files` rows to `roster`,
/// applying the client-supplied per-member wraps. Rows for users no longer
/// on the roster are dropped; the owner row is left untouched. Shared by
/// the single-file and folder-cascade move-into-shared paths.
///
/// The unique `(file_id, user_id)` index makes the upsert idempotent, so a
/// retried request lands the same row set.
pub(super) async fn rebind_node_to_roster<C: ConnectionTrait>(
    tx: &C,
    file_id: Uuid,
    member_keys: &[ParsedMemberKey],
    roster: Vec<user_files::Model>,
    caller_id: Uuid,
    signed_timestamp: i64,
) -> AppResult<()> {
    let roster_ids: HashSet<Uuid> = roster.iter().map(|r| r.user_id).collect();
    let inherited_role_by_user: HashMap<Uuid, String> = roster
        .iter()
        .map(|r| (r.user_id, r.share_role.clone()))
        .collect();
    let inherited_signature_by_user: HashMap<Uuid, Option<Vec<u8>>> = roster
        .iter()
        .map(|r| (r.user_id, r.member_signature.clone()))
        .collect();

    let existing_rows: Vec<user_files::Model> = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file_id))
        .all(tx)
        .await?;

    // Drop non-owner rows for users who left the destination's roster —
    // the per-file set is re-bound to the new folder's membership.
    for row in &existing_rows {
        if row.is_owner {
            continue;
        }
        if !roster_ids.contains(&row.user_id) {
            user_files::Entity::delete_by_id(row.id).exec(tx).await?;
        }
    }

    let existing_by_user: HashMap<Uuid, user_files::Model> = existing_rows
        .iter()
        .cloned()
        .map(|r| (r.user_id, r))
        .collect();
    let now = Utc::now().timestamp();
    for entry in member_keys {
        if entry.user_id == caller_id {
            // The owner's wrap on this node is already valid; overwriting
            // it would invalidate a working key.
            continue;
        }
        let inherited_role = inherited_role_by_user
            .get(&entry.user_id)
            .cloned()
            .unwrap_or_else(|| "reader".to_string());
        // σ_member is inherited from the destination folder's per-recipient
        // row — that signature already proves the recipient's pubkey is
        // anointed by the folder owner or a current Co-owner. The mover
        // never produces it.
        let inherited_member_signature = inherited_signature_by_user
            .get(&entry.user_id)
            .cloned()
            .flatten();
        if let Some(prev) = existing_by_user.get(&entry.user_id) {
            user_files::Entity::update(user_files::ActiveModel {
                id: ActiveValue::Unchanged(prev.id),
                encrypted_key: ActiveValue::Set(entry.encrypted_key.clone()),
                share_role: ActiveValue::Set(inherited_role),
                shared_at: ActiveValue::Set(Some(now)),
                shared_by_user_id: ActiveValue::Set(Some(caller_id)),
                member_signed_at: ActiveValue::Set(
                    inherited_member_signature.as_ref().map(|_| signed_timestamp),
                ),
                member_signature: ActiveValue::Set(inherited_member_signature),
                expires_at: ActiveValue::NotSet,
                is_owner: ActiveValue::NotSet,
                file_id: ActiveValue::Unchanged(prev.file_id),
                user_id: ActiveValue::Unchanged(prev.user_id),
                created_at: ActiveValue::NotSet,
            })
            .exec(tx)
            .await?;
        } else {
            user_files::Entity::insert(user_files::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                file_id: ActiveValue::Set(file_id),
                user_id: ActiveValue::Set(entry.user_id),
                encrypted_key: ActiveValue::Set(entry.encrypted_key.clone()),
                is_owner: ActiveValue::Set(false),
                created_at: ActiveValue::Set(now),
                expires_at: ActiveValue::Set(None),
                share_role: ActiveValue::Set(inherited_role),
                shared_at: ActiveValue::Set(Some(now)),
                shared_by_user_id: ActiveValue::Set(Some(caller_id)),
                member_signed_at: ActiveValue::Set(
                    inherited_member_signature.as_ref().map(|_| signed_timestamp),
                ),
                member_signature: ActiveValue::Set(inherited_member_signature),
            })
            .exec_without_returning(tx)
            .await?;
        }
    }
    Ok(())
}

/// Cascade move-into-shared. The caller (verified owner of the moved
/// folder) supplies one [`CascadeEntry`] per node in the subtree; the
/// server recomputes the subtree from its own state and rejects any
/// mismatch. One transaction re-parents the root and re-binds every node
/// to the destination roster.
#[allow(clippy::too_many_arguments)]
pub(super) async fn move_folder_into_shared(
    repo: &Repository<'_>,
    caller: &users::Model,
    root_id: Uuid,
    dest_id: Uuid,
    raw_entries: Vec<CascadeEntry>,
    roster: &[user_files::Model],
    signed_timestamp: i64,
    event_signature: &str,
) -> AppResult<Uuid> {
    if raw_entries.is_empty() {
        return Err(Error::BadRequest("entries_empty".to_string()));
    }
    if raw_entries.len() > ENTRIES_PER_REQUEST_CAP {
        return Err(Error::BadRequest("entries_too_many".to_string()));
    }

    let parsed = parse_cascade_entries(&raw_entries)?;
    assert_entries_match_subtree(&repo.context.db, root_id, parsed.keys()).await?;

    // The whole batch lands against one roster snapshot — every node is
    // re-wrapped for the same destination members. Verifying each node's
    // key set against that roster catches a client that wrapped a node for
    // a stale membership.
    for keys in parsed.values() {
        verify_member_keys_match(repo, dest_id, keys, roster).await?;
    }

    let tx = repo.context.db.begin().await?;

    // Only the root re-parents into the destination; descendants keep
    // their parent pointers (they move with the root by transitivity).
    files::Entity::update(files::ActiveModel {
        id: ActiveValue::Unchanged(root_id),
        file_id: ActiveValue::Set(Some(dest_id)),
        ..Default::default()
    })
    .exec(&tx)
    .await?;

    for (file_id, keys) in &parsed {
        rebind_node_to_roster(&tx, *file_id, keys, roster.to_vec(), caller.id, signed_timestamp)
            .await?;
    }

    audit::append_event(
        &tx,
        NewAuditEvent {
            sender_id: Some(caller.id),
            recipient_id: None,
            file_id: root_id,
            action_str: "shared_folder_upload",
            share_role_before: None,
            share_role_after: None,
            created_at: signed_timestamp,
            event_signature: Some(event_signature.to_string()),
        },
    )
    .await?;

    tx.commit().await?;

    Ok(root_id)
}

impl Repository<'_> {
    /// `POST /api/storage/move-out-of-shared`. The file's owner detaches
    /// their own file (or folder subtree) from the
    /// shared folder it currently lives in. Every other member's
    /// `user_files` row on the moved node(s) is dropped; the owner keeps
    /// theirs. A non-owner cannot reach this path — their "move out of my
    /// view" semantic is self-remove, not a re-parent of someone else's
    /// file.
    ///
    /// `destination_folder_id` is the new parent (NULL for the owner's
    /// drive root). It must NOT itself be a shared folder — relocating
    /// into another share is the move-into-shared re-wrap path, not this
    /// one.
    pub(crate) async fn move_out_of_shared(
        &self,
        caller: &users::Model,
        body: MoveOutOfSharedBody,
    ) -> AppResult<Uuid> {
        let body = body.validate()?;
        let file_id = Uuid::parse_str(&body.file_id.clone().unwrap())
            .map_err(|_| Error::BadRequest("file_id_invalid".to_string()))?;
        let signed_timestamp = body.timestamp.unwrap();
        let event_signature = body.event_signature.clone().unwrap();

        let dest_id = match body.destination_folder_id.as_deref() {
            Some(s) if !s.is_empty() => Some(
                Uuid::parse_str(s)
                    .map_err(|_| Error::BadRequest("destination_folder_id_invalid".to_string()))?,
            ),
            _ => None,
        };

        replay_window(signed_timestamp)?;

        let source = files::Entity::find_by_id(file_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;
        let current_parent = source
            .file_id
            .ok_or_else(|| Error::BadRequest("file_not_in_folder".to_string()))?;

        let owner_row = user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(file_id))
            .filter(user_files::Column::IsOwner.eq(true))
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::InternalError("file_has_no_owner_row".to_string()))?;

        // Only the file's owner may move it out of a shared folder. A
        // non-owner member's only exit is self-remove.
        if owner_row.user_id != caller.id {
            return Err(Error::Forbidden("cannot_move_not_owner".to_string()));
        }

        // The parent must be a folder the file genuinely sits under,
        // confirmed against the server's own tree — never the caller's
        // current read permission on it. An owner who was revoked from the
        // shared folder no longer holds a membership row there, yet must
        // still be able to pull their own file back out; gating on read
        // access would trap the file in a folder its owner can no longer
        // see.
        let parent = files::Entity::find_by_id(current_parent)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("source_folder_not_found".to_string()))?;
        if parent.mime != "dir" {
            return Err(Error::BadRequest("source_not_a_folder".to_string()));
        }
        let parent_subtree: HashSet<Uuid> =
            queries::file_tree_ids(&self.context.db, current_parent)
                .await?
                .into_iter()
                .collect();
        if !parent_subtree.contains(&file_id) {
            return Err(Error::NotFound("source_folder_not_found".to_string()));
        }

        // The destination must be a private folder the caller owns (or
        // their root). Detaching from one share straight into another
        // would skip the re-wrap the recipients of the new share need —
        // that flow is move-into-shared.
        if let Some(dest_id) = dest_id {
            let dest = files::Entity::find_by_id(dest_id)
                .one(&self.context.db)
                .await?
                .ok_or_else(|| Error::NotFound("destination_folder_not_found".to_string()))?;
            if dest.mime != "dir" {
                return Err(Error::BadRequest("destination_not_a_folder".to_string()));
            }
            let dest_owner = user_files::Entity::find()
                .filter(user_files::Column::FileId.eq(dest_id))
                .filter(user_files::Column::IsOwner.eq(true))
                .one(&self.context.db)
                .await?
                .ok_or_else(|| Error::InternalError("destination_has_no_owner_row".to_string()))?;
            if dest_owner.user_id != caller.id {
                return Err(Error::Forbidden(
                    "destination_not_owned_by_caller".to_string(),
                ));
            }
            if destination_has_other_members(self, dest_id, caller.id).await? {
                return Err(Error::BadRequest("destination_is_shared".to_string()));
            }
        }

        // Recompute the moved subtree from server state; the audit
        // canonical and the row drop both range over exactly this set.
        let subtree: Vec<Uuid> = queries::file_tree_ids(&self.context.db, file_id).await?;
        if subtree.is_empty() {
            return Err(Error::NotFound("file_not_found".to_string()));
        }
        if subtree.len() > ENTRIES_PER_REQUEST_CAP {
            return Err(Error::BadRequest("entries_too_many".to_string()));
        }

        let sig_input = AuditEventSigInputV1 {
            sender_id: caller.id.into_bytes(),
            recipient_id: None,
            file_id: file_id.into_bytes(),
            action: AuditEventActionEnum::SharedFolderMoveOut,
            share_role_before: None,
            share_role_after: None,
            timestamp: signed_timestamp,
        };
        verify_event_signature(&sig_input, &event_signature, caller)?;

        let tx = self.context.db.begin().await?;

        files::Entity::update(files::ActiveModel {
            id: ActiveValue::Unchanged(file_id),
            file_id: ActiveValue::Set(dest_id),
            ..Default::default()
        })
        .exec(&tx)
        .await?;

        // Drop every non-owner row across the moved subtree. The owner
        // keeps their rows, so the file (and its descendants) revert to
        // private files in the owner's drive.
        user_files::Entity::delete_many()
            .filter(user_files::Column::IsOwner.eq(false))
            .filter(user_files::Column::FileId.is_in(subtree))
            .exec(&tx)
            .await?;

        audit::append_event(
            &tx,
            NewAuditEvent {
                sender_id: Some(caller.id),
                recipient_id: None,
                file_id,
                action_str: "shared_folder_move_out",
                share_role_before: None,
                share_role_after: None,
                created_at: signed_timestamp,
                event_signature: Some(event_signature.clone()),
            },
        )
        .await?;

        tx.commit().await?;

        Ok(file_id)
    }
}

/// True when `folder_id` has any non-owner `user_files` row other than the
/// caller's — i.e. it is itself a shared folder.
async fn destination_has_other_members(
    repo: &Repository<'_>,
    folder_id: Uuid,
    caller_id: Uuid,
) -> AppResult<bool> {
    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .filter(user_files::Column::UserId.ne(caller_id))
        .one(&repo.context.db)
        .await?;
    Ok(row.is_some())
}

fn parse_cascade_entries(raw: &[CascadeEntry]) -> AppResult<HashMap<Uuid, Vec<ParsedMemberKey>>> {
    let mut out: HashMap<Uuid, Vec<ParsedMemberKey>> = HashMap::with_capacity(raw.len());
    for entry in raw {
        let validated = entry.clone().validate()?;
        let file_id = Uuid::from_str(&validated.file_id.unwrap())
            .map_err(|_| Error::BadRequest("entry_file_id_invalid".to_string()))?;
        let raw_keys = validated
            .member_keys
            .ok_or_else(|| Error::BadRequest("member_keys_required".to_string()))?;
        if raw_keys.is_empty() {
            return Err(Error::BadRequest("member_keys_empty".to_string()));
        }
        let keys = parse_member_keys(&raw_keys)?;
        if out.insert(file_id, keys).is_some() {
            return Err(Error::BadRequest("entry_file_id_duplicate".to_string()));
        }
    }
    Ok(out)
}

async fn assert_entries_match_subtree<'a, C: ConnectionTrait, I>(
    db: &C,
    root_id: Uuid,
    supplied_ids: I,
) -> AppResult<()>
where
    I: Iterator<Item = &'a Uuid>,
{
    let subtree: HashSet<Uuid> = queries::file_tree_ids(db, root_id).await?.into_iter().collect();
    if subtree.is_empty() {
        return Err(Error::NotFound("file_not_found".to_string()));
    }
    let supplied: HashSet<Uuid> = supplied_ids.copied().collect();
    if supplied != subtree {
        return Err(Error::BadRequest("entries_do_not_match_subtree".to_string()));
    }
    Ok(())
}
