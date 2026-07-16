//! Core CRUD path for `POST /api/shares` and `DELETE /api/shares`. All
//! validation, signature verification, and audit-row insertion runs in a
//! single transaction so a failed signature check rolls back any partial
//! work.

use std::collections::{HashMap, HashSet};

use cryptfns::asn1::{
    decode_share_request_v1, encode_audit_event_sig_input_v1, encode_entries_v1,
    AuditEventActionEnum, AuditEventSigInputV1, ShareEntry, ShareRoleEnum, SHARE_REQUEST_V1_PREFIX,
};
use entity::{
    files,
    permission::{permission, SharePermission},
    user_files, users, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};
use sha2::{Digest, Sha256};

use crate::{
    contracts::audit::NewAuditEvent,
    data::create_share::{
        CreateShareEntry, CreateShareEnvelope, FolderMemberListSig, RevokeShareBody,
    },
    repository::{
        audit,
        members_list_sig::{self, prospective_from_db, MembersListSig},
        nonce, notify, queries, rate_limit,
        share_sig::{
            fingerprint_bytes, role_enum_to_str, role_str_to_enum, static_role_str,
            verify_member_signature,
        },
        Repository,
    },
};
use validr::Validation;

const REPLAY_WINDOW_SECONDS: i64 = 300;
const ENTRIES_PER_REQUEST_CAP: usize = 5000;

/// Outcome of [`Repository::create_share`] — one [`AppShare`] per `entries`
/// item that landed in the database.
pub(crate) struct CreateShareOutput {
    pub shares: Vec<crate::data::app_share::AppShare>,
}

/// Outcome of [`Repository::revoke_share`]. Both variants resolve to HTTP
/// 204 on the route side; the distinction exists so future surfaces (the
/// audit-log UI's "what happened" pane, for instance) can distinguish a
/// real revocation from an idempotent no-op.
pub(crate) enum RevokeOutput {
    Removed,
    Idempotent,
}

impl<'ctx> Repository<'ctx> {
    /// Validate the envelope, verify both signatures (over the bytes
    /// received), and upsert one row per entry plus a single audit-log
    /// `grant` or `role_change` row. Email dispatch is intentionally left
    /// to the caller so the integration tests can assert without spinning
    /// up an SMTP mock.
    pub(crate) async fn create_share(
        &self,
        envelope: CreateShareEnvelope,
        sender: &users::Model,
    ) -> AppResult<CreateShareOutput> {
        let envelope = envelope.validate()?;
        let payload_der_b64 = envelope.payload_der.clone().unwrap();
        let signature_b64 = envelope.signature.clone().unwrap();
        let event_signature_b64 = envelope.event_signature.clone().unwrap();
        let raw_entries = envelope.entries.unwrap_or_default();
        let members_list_sig_input = envelope.members_list_signature.clone();
        let supplied_member_sig = envelope.member_signature.clone();
        let supplied_member_signed_at = envelope.member_signed_at;

        if raw_entries.is_empty() {
            return Err(Error::BadRequest("entries_empty".to_string()));
        }
        if raw_entries.len() > ENTRIES_PER_REQUEST_CAP {
            return Err(Error::BadRequest("entries_too_many".to_string()));
        }

        let payload_der = cryptfns::base64::decode(&payload_der_b64)
            .map_err(|_| Error::BadRequest("payload_der_invalid_base64".to_string()))?;
        let payload = decode_share_request_v1(&payload_der)
            .map_err(|_| Error::BadRequest("payload_der_invalid".to_string()))?;

        let sender_id_decoded = Uuid::from_bytes(payload.sender_id);
        if sender_id_decoded != sender.id {
            return Err(Error::Forbidden("sender_id_mismatch".to_string()));
        }

        let recipient_id = Uuid::from_bytes(payload.recipient_id);
        if recipient_id == sender.id {
            return Err(Error::BadRequest("cannot_share_with_self".to_string()));
        }

        let now = chrono::Utc::now().timestamp();
        if (now - payload.timestamp).abs() > REPLAY_WINDOW_SECONDS {
            return Err(Error::BadRequest("replay_timestamp_skew".to_string()));
        }
        if nonce::check_and_record(sender.id, payload.nonce, now) {
            return Err(Error::Conflict("replay_nonce_seen".to_string()));
        }

        let recipient = users::Entity::find_by_id(recipient_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("recipient_not_found".to_string()))?;
        let recipient_fingerprint_bytes = fingerprint_bytes(&recipient.fingerprint)?;
        if recipient_fingerprint_bytes != payload.recipient_pubkey_fingerprint {
            return Err(Error::Conflict("recipient_pubkey_changed".to_string()));
        }

        let root_file_id = Uuid::from_bytes(payload.root_file_id);
        let entries = parse_entries(&raw_entries)?;
        let entries_hash_bytes: Vec<(Uuid, Vec<u8>)> = entries
            .iter()
            .map(|(file_id, encrypted_key)| {
                let bytes = cryptfns::base64::decode(encrypted_key)
                    .map_err(|_| Error::BadRequest("entry_encrypted_key_invalid_base64".to_string()))?;
                Ok::<_, Error>((*file_id, bytes))
            })
            .collect::<AppResult<Vec<_>>>()?;
        verify_entries_match_subtree(&self.context.db, root_file_id, &entries).await?;

        let caller_perm = permission(&self.context.db, root_file_id, sender.id).await?;
        let requested_role = payload.share_role;
        match caller_perm {
            SharePermission::Owner => {}
            SharePermission::CoOwner => {
                if matches!(requested_role, ShareRoleEnum::CoOwner) {
                    return Err(Error::Forbidden("cannot_grant_equal_role".to_string()));
                }
            }
            _ => {
                return Err(Error::Forbidden("cannot_share_not_owner".to_string()));
            }
        }

        // Folder shares mutate the folder's member list — the request
        // must carry a fresh `members_list_signature` over the post-
        // share roster. Non-folder shares (regular file shares) don't
        // touch the folder list and skip this branch entirely.
        let root_file = files::Entity::find_by_id(root_file_id)
            .one(&self.context.db)
            .await?;
        let root_is_folder = root_file
            .as_ref()
            .map(|f| f.mime == "dir")
            .unwrap_or(false);
        let parsed_list_sig: Option<MembersListSig> = if root_is_folder {
            let raw = members_list_sig_input
                .clone()
                .ok_or_else(|| Error::BadRequest("missing_members_list_signature".to_string()))?;
            Some(parse_list_sig(raw)?)
        } else {
            None
        };

        let entries_for_hash: Vec<ShareEntry> = entries_hash_bytes
            .iter()
            .map(|(file_id, encrypted_key_raw)| ShareEntry {
                file_id: file_id.into_bytes(),
                encrypted_key: encrypted_key_raw.clone(),
            })
            .collect();
        let entries_der = encode_entries_v1(&entries_for_hash)
            .map_err(|e| Error::CryptoError(Box::new(e)))?;
        let mut hasher = Sha256::new();
        hasher.update(&entries_der);
        let computed_entries_hash: [u8; 32] = hasher.finalize().into();
        if computed_entries_hash != payload.entries_hash {
            return Err(Error::BadRequest("entries_hash_mismatch".to_string()));
        }

        let mut signing_input = Vec::with_capacity(SHARE_REQUEST_V1_PREFIX.len() + payload_der.len());
        signing_input.extend_from_slice(SHARE_REQUEST_V1_PREFIX);
        signing_input.extend_from_slice(&payload_der);
        cryptfns::rsa::public::verify_bytes(&signing_input, &signature_b64, &sender.pubkey)
            .map_err(|_| Error::BadRequest("invalid_signature".to_string()))?;

        // Per-recipient `MemberSigPayloadV1` signature, when supplied.
        // The producer signs the recipient's pubkey + fingerprint + role
        // at issue time with the granter's privkey, so any later viewer
        // of the member list can chain trust from owner → Co-owner.
        // Verification re-encodes the payload from the recipient's row
        // and `member_signed_at`, then RSA-PSS-verifies the supplied
        // signature against the granter's pubkey.
        // The signature lands verbatim in every produced `user_files`
        // row's `member_signature` column.
        // Schema persists `user_files.member_signature` as raw bytes.
        // The wire field is base64 for transport;
        // server decodes once during verification and stores the
        // decoded form. Downstream readers (`folder_members` route,
        // `verify_post_mutation_signature`) base64-encode again before
        // serialising to clients.
        let member_sig_signed_at = supplied_member_signed_at;
        let persisted_member_sig: Option<Vec<u8>> = match supplied_member_sig.as_ref() {
            Some(sig_b64) => {
                let signed_at = member_sig_signed_at.ok_or_else(|| {
                    Error::BadRequest("member_signature_missing_signed_at".to_string())
                })?;
                if (now - signed_at).abs() > REPLAY_WINDOW_SECONDS {
                    return Err(Error::BadRequest(
                        "member_signature_timestamp_skew".to_string(),
                    ));
                }
                Some(verify_member_signature(
                    &sender.pubkey,
                    sig_b64,
                    recipient.id,
                    &recipient.pubkey,
                    recipient_fingerprint_bytes,
                    requested_role,
                    signed_at,
                )?)
            }
            None => None,
        };
        // The signed timestamp goes in its own column so `shared_at` stays the
        // server-side share time that orders the recipient's shares list. It's
        // set only when a σ is actually persisted.
        let member_signed_at_col = if persisted_member_sig.is_some() {
            member_sig_signed_at
        } else {
            None
        };

        let requested_role_str = role_enum_to_str(requested_role);

        // The audit row's action depends on whether the recipient already
        // holds a row on the root file: a fresh grant (or Co-owner reshare)
        // when not, a `role_change` when the role is moving. The audit
        // signature must cover that action plus the before/after roles,
        // so we look up the existing rows BEFORE verifying the signature
        // and use the resolved action when reconstructing the canonical
        // input. Persisting a `grant`-signed signature on a `role_change`
        // row (the prior shape) left every legitimate upgrade unverifiable.
        let existing = user_files::Entity::find()
            .filter(user_files::Column::FileId.is_in(entries.iter().map(|e| e.0)))
            .filter(user_files::Column::UserId.eq(recipient.id))
            .all(&self.context.db)
            .await?;
        let existing_by_file: HashMap<Uuid, user_files::Model> =
            existing.into_iter().map(|m| (m.file_id, m)).collect();

        let root_previous_role = existing_by_file
            .get(&root_file_id)
            .filter(|prev| !prev.is_owner && prev.share_role != requested_role_str)
            .map(|prev| prev.share_role.clone());
        let root_previous_role_enum = root_previous_role
            .as_deref()
            .and_then(role_str_to_enum);

        let (action, action_str): (AuditEventActionEnum, &'static str) =
            if root_previous_role.is_some() {
                (AuditEventActionEnum::RoleChange, "role_change")
            } else {
                match caller_perm {
                    SharePermission::CoOwner => (
                        AuditEventActionEnum::SharedByCoOwner,
                        "shared_by_co_owner",
                    ),
                    _ => (AuditEventActionEnum::Grant, "grant"),
                }
            };
        let sig_input = AuditEventSigInputV1 {
            sender_id: sender.id.into_bytes(),
            recipient_id: Some(recipient.id.into_bytes()),
            file_id: root_file_id.into_bytes(),
            action,
            share_role_before: root_previous_role_enum,
            share_role_after: Some(requested_role),
            timestamp: payload.timestamp,
        };
        let sig_input_der = encode_audit_event_sig_input_v1(&sig_input)
            .map_err(|e| Error::CryptoError(Box::new(e)))?;
        let mut audit_signing_input =
            Vec::with_capacity(cryptfns::asn1::AUDIT_EVENT_SIG_V1_PREFIX.len() + sig_input_der.len());
        audit_signing_input.extend_from_slice(cryptfns::asn1::AUDIT_EVENT_SIG_V1_PREFIX);
        audit_signing_input.extend_from_slice(&sig_input_der);
        cryptfns::rsa::public::verify_bytes(
            &audit_signing_input,
            &event_signature_b64,
            &sender.pubkey,
        )
        .map_err(|_| Error::BadRequest("event_signature_invalid".to_string()))?;

        if rate_limit::over_per_pair_cap(
            &self.context.db,
            sender.id,
            recipient.id,
            root_file_id,
            now,
        )
        .await?
        {
            return Err(Error::TooManyRequests(
                "per_recipient_cap_exceeded".to_string(),
            ));
        }

        let tx = self.context.db.begin().await?;
        let mut produced: Vec<Uuid> = Vec::with_capacity(entries.len());
        let mut role_changes: Vec<(Uuid, String)> = Vec::new();
        for (file_id, encrypted_key) in &entries {
            if let Some(prev) = existing_by_file.get(file_id) {
                if prev.is_owner {
                    return Err(Error::BadRequest(
                        "cannot_share_owner_row".to_string(),
                    ));
                }
                let previous_role = prev.share_role.clone();
                if previous_role == requested_role_str {
                    continue;
                }
                // Refresh the per-row σ whenever the granter supplied a
                // fresh one — a role-change is logically a re-grant and
                // the new role is what σ commits to. When the caller
                // omitted σ (legacy client), leave the
                // previous σ in place so a partial upgrade doesn't tear
                // down already-valid signatures.
                let (role_change_member_sig, role_change_member_signed_at) =
                    match persisted_member_sig.as_ref() {
                        Some(sig) => (
                            ActiveValue::Set(Some(sig.clone())),
                            ActiveValue::Set(member_signed_at_col),
                        ),
                        None => (ActiveValue::NotSet, ActiveValue::NotSet),
                    };
                let active = user_files::ActiveModel {
                    id: ActiveValue::Unchanged(prev.id),
                    encrypted_key: ActiveValue::Set(encrypted_key.clone()),
                    share_role: ActiveValue::Set(requested_role_str.to_string()),
                    shared_at: ActiveValue::Set(Some(now)),
                    shared_by_user_id: ActiveValue::Set(Some(sender.id)),
                    member_signature: role_change_member_sig,
                    member_signed_at: role_change_member_signed_at,
                    expires_at: ActiveValue::NotSet,
                    is_owner: ActiveValue::NotSet,
                    file_id: ActiveValue::Unchanged(prev.file_id),
                    user_id: ActiveValue::Unchanged(prev.user_id),
                    created_at: ActiveValue::NotSet,
                };
                user_files::Entity::update(active).exec(&tx).await?;
                produced.push(*file_id);
                role_changes.push((*file_id, previous_role));
            } else {
                let active = user_files::ActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    file_id: ActiveValue::Set(*file_id),
                    user_id: ActiveValue::Set(recipient.id),
                    encrypted_key: ActiveValue::Set(encrypted_key.clone()),
                    is_owner: ActiveValue::Set(false),
                    created_at: ActiveValue::Set(now),
                    expires_at: ActiveValue::Set(None),
                    share_role: ActiveValue::Set(requested_role_str.to_string()),
                    shared_at: ActiveValue::Set(Some(now)),
                    shared_by_user_id: ActiveValue::Set(Some(sender.id)),
                    member_signature: ActiveValue::Set(persisted_member_sig.clone()),
                    member_signed_at: ActiveValue::Set(member_signed_at_col),
                };
                user_files::Entity::insert(active)
                    .exec_without_returning(&tx)
                    .await?;
                produced.push(*file_id);
            }
        }

        if !produced.is_empty() {
            // Membership-mutation bookkeeping for the folder member-list
            // protocol. The TOCTOU check in `upload-multikey` /
            // `move-into-shared` requires the uploader's snapshot to be
            // at-or-newer-than this stamp. We
            // bump it on every share that changes the folder's member
            // set; idempotent calls (zero entries produced) don't bump.
            files::Entity::update(files::ActiveModel {
                id: ActiveValue::Unchanged(root_file_id),
                last_membership_change_at: ActiveValue::Set(Some(payload.timestamp)),
                ..Default::default()
            })
            .exec(&tx)
            .await?;

            // The audit row's `created_at` is the timestamp that was
            // covered by `event_signature` — re-verifying that signature
            // later requires reconstructing the exact signed input.
            // Replay protection (timestamp ±5 min) is the
            // separate check that already bounded `payload.timestamp` to
            // the server's clock. The action + before/after roles match
            // the canonical the audit signature was verified against
            // above, so a later verifier produces the same DER and the
            // RSA-PSS check passes.
            audit::append_event(
                &tx,
                NewAuditEvent {
                    sender_id: Some(sender.id),
                    recipient_id: Some(recipient.id),
                    file_id: root_file_id,
                    action_str,
                    share_role_before: root_previous_role
                        .as_deref()
                        .and_then(static_role_str),
                    share_role_after: Some(requested_role_str),
                    created_at: payload.timestamp,
                    event_signature: Some(event_signature_b64.clone()),
                },
            )
            .await?;
        }

        if let Some(sig) = parsed_list_sig.as_ref() {
            // Read the in-flight roster from inside the tx so the
            // signature covers exactly the committed-to set. On
            // idempotent calls (no produced rows) the live state still
            // reflects the current member set and the client's
            // signature is expected to cover that same set.
            let (owner_id, members_after) =
                prospective_from_db(&tx, root_file_id).await?;
            members_list_sig::verify_post_mutation_signature(
                &tx,
                root_file_id,
                owner_id,
                &members_after,
                sig,
                now,
            )
            .await?;
            members_list_sig::store_signature(&tx, root_file_id, sig).await?;
        }

        tx.commit().await?;

        // Notify the recipient by email only when at least one row was
        // freshly inserted; pure role-change updates aren't a new share
        // from the recipient's point of view.
        let has_new_grant = produced.len() > role_changes.len();
        if has_new_grant {
            notify::share_created(self.context, sender, &recipient).await;
        }

        let recipient_rows = user_files::Entity::find()
            .filter(user_files::Column::FileId.is_in(produced.iter().copied()))
            .filter(user_files::Column::UserId.eq(recipient.id))
            .filter(user_files::Column::IsOwner.eq(false))
            .all(&self.context.db)
            .await?;
        let sender_email = sender.email.clone();
        let shares = recipient_rows
            .into_iter()
            .map(|row| crate::data::app_share::AppShare {
                file_id: row.file_id,
                recipient_id: recipient.id,
                recipient_email: recipient.email.clone(),
                recipient_pubkey_fingerprint: recipient.fingerprint.clone(),
                share_role: row.share_role,
                created_at: row.created_at,
                shared_at: row.shared_at,
                shared_by_user_id: row.shared_by_user_id,
                shared_by_email: Some(sender_email.clone()),
            })
            .collect();

        Ok(CreateShareOutput { shares })
    }

    /// Authorise + drop the recipient's row for `file_id`, recursing into
    /// the folder tree. When the revoked recipient was a Co-owner, also
    /// drop every row they had granted under the same scope, with a
    /// `shared_by_co_owner_revoked` system audit row per cascaded entry.
    pub(crate) async fn revoke_share(
        &self,
        body: RevokeShareBody,
        caller: &users::Model,
        file_id: Uuid,
        recipient_id: Uuid,
    ) -> AppResult<RevokeOutput> {
        let members_list_sig_input = body.members_list_signature.clone();
        let body = body.validate()?;
        let event_signature_b64 = body.event_signature.clone().unwrap();
        let signed_timestamp = body.timestamp.unwrap();

        // A recipient dropping their own row is the documented exit and
        // shares the same DELETE route with owner/co-owner revoke. It
        // bypasses the owner/co-owner-grantor
        // gate below and the post-mutation members-list signature: the
        // leaving recipient is neither authorised to sign the new
        // roster nor responsible for proving it.
        let is_self_remove = recipient_id == caller.id;

        let now = chrono::Utc::now().timestamp();
        if (now - signed_timestamp).abs() > REPLAY_WINDOW_SECONDS {
            return Err(Error::BadRequest("replay_timestamp_skew".to_string()));
        }

        let target = user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(file_id))
            .filter(user_files::Column::UserId.eq(recipient_id))
            .filter(user_files::Column::IsOwner.eq(false))
            .one(&self.context.db)
            .await?;
        let Some(target) = target else {
            return Ok(RevokeOutput::Idempotent);
        };

        if !is_self_remove {
            let caller_perm = permission(&self.context.db, file_id, caller.id).await?;
            if !caller_perm.can_reshare() {
                return Err(Error::Forbidden("cannot_revoke_not_owner".to_string()));
            }
        }

        // The signed input matches the timestamp the client supplied in
        // the body; that timestamp was already replay-window-checked
        // above. Reconstructing it from a fresh server clock would
        // produce a different signing input and break verification when
        // request handling crosses a second boundary.
        let sig_input = AuditEventSigInputV1 {
            sender_id: caller.id.into_bytes(),
            recipient_id: Some(recipient_id.into_bytes()),
            file_id: file_id.into_bytes(),
            action: AuditEventActionEnum::Revoke,
            share_role_before: role_str_to_enum(&target.share_role),
            share_role_after: None,
            timestamp: signed_timestamp,
        };
        let sig_input_der = encode_audit_event_sig_input_v1(&sig_input)
            .map_err(|e| Error::CryptoError(Box::new(e)))?;
        let mut audit_signing_input =
            Vec::with_capacity(cryptfns::asn1::AUDIT_EVENT_SIG_V1_PREFIX.len() + sig_input_der.len());
        audit_signing_input.extend_from_slice(cryptfns::asn1::AUDIT_EVENT_SIG_V1_PREFIX);
        audit_signing_input.extend_from_slice(&sig_input_der);
        cryptfns::rsa::public::verify_bytes(
            &audit_signing_input,
            &event_signature_b64,
            &caller.pubkey,
        )
        .map_err(|_| Error::BadRequest("event_signature_invalid".to_string()))?;

        let revoked_role_before = target.share_role.clone();
        let revoked_was_co_owner = revoked_role_before == "co-owner";

        // Folder revoke must carry a fresh list signature over the
        // post-revoke roster. Non-folder revoke skips.
        let target_file = files::Entity::find_by_id(file_id)
            .one(&self.context.db)
            .await?;
        let target_is_folder = target_file
            .as_ref()
            .map(|f| f.mime == "dir")
            .unwrap_or(false);
        let parsed_list_sig: Option<MembersListSig> = if target_is_folder && !is_self_remove {
            let raw = members_list_sig_input
                .clone()
                .ok_or_else(|| Error::BadRequest("missing_members_list_signature".to_string()))?;
            Some(parse_list_sig(raw)?)
        } else {
            None
        };

        let scope_ids: HashSet<Uuid> = queries::file_tree_ids(&self.context.db, file_id)
            .await?
            .into_iter()
            .collect();

        let tx = self.context.db.begin().await?;

        // Primary revocation: every row that names the revoked recipient
        // anywhere under the scope. For a folder share this drops the
        // recipient's rows on the root and on every descendant in one go.
        let primary_rows: Vec<user_files::Model> = user_files::Entity::find()
            .filter(user_files::Column::UserId.eq(recipient_id))
            .filter(user_files::Column::IsOwner.eq(false))
            .filter(user_files::Column::FileId.is_in(scope_ids.iter().copied().collect::<Vec<_>>()))
            .all(&tx)
            .await?;
        for row in &primary_rows {
            user_files::Entity::delete_by_id(row.id).exec(&tx).await?;
        }

        // Membership changed — bump the folder's stamp so any in-flight
        // multi-key upload that still references the pre-revoke roster
        // gets the TOCTOU 409 from `upload-multikey`.
        if !primary_rows.is_empty() {
            files::Entity::update(files::ActiveModel {
                id: ActiveValue::Unchanged(file_id),
                last_membership_change_at: ActiveValue::Set(Some(signed_timestamp)),
                ..Default::default()
            })
            .exec(&tx)
            .await?;
        }

        // Append the primary revoke audit row signed by the caller —
        // `created_at` matches the timestamp covered by the signature
        // for the same reason as the create path.
        audit::append_event(
            &tx,
            NewAuditEvent {
                sender_id: Some(caller.id),
                recipient_id: Some(recipient_id),
                file_id,
                action_str: "revoke",
                share_role_before: static_role_str(&revoked_role_before),
                share_role_after: None,
                created_at: signed_timestamp,
                event_signature: Some(event_signature_b64.clone()),
            },
        )
        .await?;

        if revoked_was_co_owner {
            let cascade_rows: Vec<user_files::Model> = user_files::Entity::find()
                .filter(user_files::Column::SharedByUserId.eq(recipient_id))
                .filter(user_files::Column::IsOwner.eq(false))
                .filter(
                    user_files::Column::FileId
                        .is_in(scope_ids.iter().copied().collect::<Vec<_>>()),
                )
                .all(&tx)
                .await?;

            for row in &cascade_rows {
                let role_before = row.share_role.clone();
                let downstream_recipient = row.user_id;
                let downstream_file_id = row.file_id;
                user_files::Entity::delete_by_id(row.id).exec(&tx).await?;
                // Cascade rows are system-attributed (no signature to
                // match), so the server's clock is fine for created_at.
                audit::append_event(
                    &tx,
                    NewAuditEvent {
                        sender_id: None,
                        recipient_id: Some(downstream_recipient),
                        file_id: downstream_file_id,
                        action_str: "shared_by_co_owner_revoked",
                        share_role_before: static_role_str(&role_before),
                        share_role_after: None,
                        created_at: now,
                        event_signature: None,
                    },
                )
                .await?;
            }

            // Files the departing co-owner uploaded into the shared folder
            // are theirs (`is_owner = true`); they survive both deletes
            // above. Left in place they would dangle under a folder their
            // owner can no longer reach. Relocate them to the owner's root
            // (parent NULL) and drop every other member's row on them, so
            // they revert to private files in the departing owner's drive —
            // mirroring move-out / evict. Only the top of each owned subtree
            // re-parents; owned descendants keep their pointers and move with
            // their parent by transitivity.
            let owned_ids: HashSet<Uuid> =
                queries::file_tree_owner_ids(&tx, file_id, recipient_id)
                    .await?
                    .into_iter()
                    .collect();
            if !owned_ids.is_empty() {
                let owned_id_vec: Vec<Uuid> = owned_ids.iter().copied().collect();
                user_files::Entity::delete_many()
                    .filter(user_files::Column::IsOwner.eq(false))
                    .filter(user_files::Column::FileId.is_in(owned_id_vec.clone()))
                    .exec(&tx)
                    .await?;

                let owned_files = files::Entity::find()
                    .filter(files::Column::Id.is_in(owned_id_vec))
                    .all(&tx)
                    .await?;
                for file in &owned_files {
                    let parent_owned = file.file_id.map(|p| owned_ids.contains(&p)).unwrap_or(false);
                    if parent_owned {
                        continue;
                    }
                    files::Entity::update(files::ActiveModel {
                        id: ActiveValue::Unchanged(file.id),
                        file_id: ActiveValue::Set(None),
                        ..Default::default()
                    })
                    .exec(&tx)
                    .await?;
                    audit::append_event(
                        &tx,
                        NewAuditEvent {
                            sender_id: None,
                            recipient_id: Some(recipient_id),
                            file_id: file.id,
                            action_str: "relocated_on_revoke",
                            share_role_before: None,
                            share_role_after: None,
                            created_at: now,
                            event_signature: None,
                        },
                    )
                    .await?;
                }
            }
        }

        if let Some(sig) = parsed_list_sig.as_ref() {
            // The primary delete above already pruned the revoked
            // recipient's row, so the live read reflects the post-
            // revoke set. The signer must still be in that set (or be
            // the owner) — `verify_post_mutation_signature` enforces
            // authorisation.
            let (owner_id, members_after) = prospective_from_db(&tx, file_id).await?;
            members_list_sig::verify_post_mutation_signature(
                &tx,
                file_id,
                owner_id,
                &members_after,
                sig,
                now,
            )
            .await?;
            members_list_sig::store_signature(&tx, file_id, sig).await?;
        }

        tx.commit().await?;

        Ok(RevokeOutput::Removed)
    }

    /// Recipient list for `file_id`, visible to every member of the share.
    /// Sharing a file shares the roster: each member sees who else is on
    /// it so the trust model is transparent. Add / remove / role-change
    /// remain gated by `can_reshare` at the mutating routes.
    pub(crate) async fn recipient_list(
        &self,
        caller: &users::Model,
        file_id: Uuid,
    ) -> AppResult<Vec<crate::data::app_share::AppShare>> {
        let perm = permission(&self.context.db, file_id, caller.id).await?;
        if matches!(perm, SharePermission::None) {
            return Err(Error::NotFound("file_not_found".to_string()));
        }
        queries::recipient_list(&self.context.db, file_id).await
    }
}

fn parse_entries(raw: &[CreateShareEntry]) -> AppResult<Vec<(Uuid, String)>> {
    let mut seen: HashSet<Uuid> = HashSet::with_capacity(raw.len());
    let mut out = Vec::with_capacity(raw.len());
    for entry in raw {
        let validated = entry.clone().validate()?;
        let file_id_str = validated.file_id.unwrap();
        let encrypted_key = validated.encrypted_key.unwrap();
        let file_id = Uuid::parse_str(&file_id_str)
            .map_err(|_| Error::BadRequest("entry_file_id_invalid".to_string()))?;
        if !seen.insert(file_id) {
            return Err(Error::BadRequest("entry_file_id_duplicate".to_string()));
        }
        if encrypted_key.is_empty() {
            return Err(Error::BadRequest("entry_encrypted_key_empty".to_string()));
        }
        out.push((file_id, encrypted_key));
    }
    Ok(out)
}

async fn verify_entries_match_subtree<C: ConnectionTrait>(
    db: &C,
    root_file_id: Uuid,
    entries: &[(Uuid, String)],
) -> AppResult<()> {
    let subtree = queries::file_tree_ids(db, root_file_id).await?;
    let subtree_set: HashSet<Uuid> = subtree.into_iter().collect();
    if subtree_set.is_empty() {
        return Err(Error::NotFound("root_file_not_found".to_string()));
    }
    let supplied: HashSet<Uuid> = entries.iter().map(|e| e.0).collect();
    if supplied != subtree_set {
        return Err(Error::BadRequest(
            "entries_do_not_match_subtree".to_string(),
        ));
    }
    Ok(())
}

fn parse_list_sig(raw: FolderMemberListSig) -> AppResult<MembersListSig> {
    let signed_by = Uuid::parse_str(&raw.signed_by_user_id).map_err(|_| {
        Error::BadRequest("members_list_signed_by_user_id_invalid".to_string())
    })?;
    Ok(MembersListSig {
        signature_b64: raw.signature,
        signed_at: raw.signed_at,
        signed_by_user_id: signed_by,
    })
}
