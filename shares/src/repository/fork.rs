//! `POST /api/shares/{file_id}/fork` — save-to-my-drive. The client has
//! decrypted the source file with their existing wrap, generated a fresh
//! symmetric key, and re-encrypted the content; this endpoint creates the
//! new file row + per-user owner row on the caller's drive, gated by
//! `permission().can_fork()` plus a quota check.

use chrono::Utc;
use cryptfns::asn1::{
    encode_audit_event_sig_input_v1, AuditEventActionEnum, AuditEventSigInputV1,
    AUDIT_EVENT_SIG_V1_PREFIX,
};
use entity::{
    files,
    permission::{permission, SharePermission},
    user_files, users, ActiveValue, EntityTrait, TransactionTrait, Uuid,
};
use error::{AppResult, Error};

use crate::{
    contracts::audit::NewAuditEvent,
    data::fork::ForkBody,
    repository::{audit, Repository},
};
use validr::Validation;

const REPLAY_WINDOW_SECONDS: i64 = 300;

pub(crate) struct ForkOutput {
    pub new_file_id: Uuid,
    pub created_at: i64,
}

impl Repository<'_> {
    pub(crate) async fn fork_file(
        &self,
        caller: &users::Model,
        source_file_id: Uuid,
        body: ForkBody,
    ) -> AppResult<ForkOutput> {
        let validated = body.validate()?;
        let new_file_id = Uuid::parse_str(&validated.new_file_id.unwrap())
            .map_err(|_| Error::BadRequest("new_file_id_invalid".to_string()))?;
        if new_file_id == source_file_id {
            return Err(Error::BadRequest("new_file_id_must_differ".to_string()));
        }
        let signed_timestamp = validated.timestamp.unwrap();
        let event_signature_b64 = validated.event_signature.unwrap();
        let encrypted_key = validated.encrypted_key.unwrap();
        let encrypted_metadata = validated.encrypted_metadata.unwrap();
        let name_hash = validated.name_hash.unwrap();
        let mime = validated.mime.unwrap();
        if mime == "dir" {
            return Err(Error::BadRequest("cannot_fork_directory".to_string()));
        }

        let now = Utc::now().timestamp();
        if (now - signed_timestamp).abs() > REPLAY_WINDOW_SECONDS {
            return Err(Error::BadRequest("replay_timestamp_skew".to_string()));
        }

        let perm = permission(&self.context.db, source_file_id, caller.id).await?;
        match perm {
            SharePermission::Owner | SharePermission::CoOwner => {}
            SharePermission::None => {
                return Err(Error::NotFound("file_not_found".to_string()));
            }
            _ => return Err(Error::Forbidden("forbidden_not_forkable".to_string())),
        }

        let source = files::Entity::find_by_id(source_file_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;
        if source.mime == "dir" {
            return Err(Error::BadRequest("cannot_fork_directory".to_string()));
        }

        // Quota: the new file counts against the caller's
        // owner-attributed bytes. Run after permission so a
        // 403 caller never learns whether they'd have hit a quota wall.
        // Resolution mirrors `storage::routes::create::create`: a per-
        // user `users.quota` overrides; otherwise `Settings.users.
        // quota_bytes` applies; otherwise unlimited.
        let claimed_size = validated.size.unwrap_or(0);
        let effective_quota: Option<i64> = match caller.quota {
            Some(v) => Some(v),
            None => self
                .context
                .settings
                .inner()
                .await
                .users
                .quota_bytes()
                .map(|u| u as i64),
        };
        if let Some(quota) = effective_quota {
            let used = super::multikey_upload::owner_used_space(self, caller.id).await?;
            if used + claimed_size > quota {
                return Err(Error::Conflict("fork_quota_exceeded".to_string()));
            }
        }

        // The audit row is attributed to the source file id so the owner
        // of the original sees who saved a copy. The signed input mirrors
        // that — the client signs over `source_file_id` and the server
        // verifies the same bytes.
        let sig_input = AuditEventSigInputV1 {
            sender_id: caller.id.into_bytes(),
            recipient_id: None,
            file_id: source_file_id.into_bytes(),
            action: AuditEventActionEnum::Fork,
            share_role_before: None,
            share_role_after: None,
            timestamp: signed_timestamp,
        };
        let sig_der = encode_audit_event_sig_input_v1(&sig_input)
            .map_err(|e| Error::CryptoError(Box::new(e)))?;
        let mut signing_input =
            Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + sig_der.len());
        signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
        signing_input.extend_from_slice(&sig_der);
        cryptfns::rsa::public::verify_bytes(&signing_input, &event_signature_b64, &caller.pubkey)
            .map_err(|_| Error::BadRequest("event_signature_invalid".to_string()))?;

        let tx = self.context.db.begin().await?;

        let file_active = files::ActiveModel {
            id: ActiveValue::Set(new_file_id),
            name_hash: ActiveValue::Set(name_hash),
            encrypted_name: ActiveValue::Set(encrypted_metadata),
            encrypted_thumbnail: ActiveValue::Set(validated.encrypted_thumbnail),
            mime: ActiveValue::Set(mime),
            size: ActiveValue::Set(validated.size),
            chunks: ActiveValue::Set(validated.chunks),
            chunks_stored: ActiveValue::Set(Some(0)),
            // The fork lives at the caller's drive root by design.
            // The user can move it later via the existing move-many
            // route.
            file_id: ActiveValue::Set(None),
            md5: ActiveValue::Set(validated.md5),
            sha1: ActiveValue::Set(validated.sha1),
            sha256: ActiveValue::Set(validated.sha256),
            blake2b: ActiveValue::Set(validated.blake2b),
            cipher: ActiveValue::Set(validated.cipher.unwrap_or_else(|| "ascon128a".to_string())),
            editable: ActiveValue::Set(source.editable),
            file_modified_at: ActiveValue::Set(signed_timestamp),
            created_at: ActiveValue::Set(signed_timestamp),
            finished_upload_at: ActiveValue::Set(None),
            active_version: ActiveValue::Set(1),
            pending_version: ActiveValue::Set(None),
            pending_chunks: ActiveValue::Set(None),
            pending_size: ActiveValue::Set(None),
            last_membership_change_at: ActiveValue::Set(None),
            members_list_signature: ActiveValue::Set(None),
            members_list_signed_at: ActiveValue::Set(None),
            members_list_signed_by_user_id: ActiveValue::Set(None),
        };
        files::Entity::insert(file_active)
            .exec_without_returning(&tx)
            .await?;

        let user_file = user_files::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            file_id: ActiveValue::Set(new_file_id),
            user_id: ActiveValue::Set(caller.id),
            encrypted_key: ActiveValue::Set(encrypted_key),
            is_owner: ActiveValue::Set(true),
            created_at: ActiveValue::Set(signed_timestamp),
            expires_at: ActiveValue::Set(None),
            // Owner rows carry `co-owner` by convention; permission()
            // ignores `share_role` when `is_owner=true`.
            share_role: ActiveValue::Set("co-owner".to_string()),
            shared_at: ActiveValue::Set(None),
            shared_by_user_id: ActiveValue::Set(None),
            member_signature: ActiveValue::Set(None),
        };
        user_files::Entity::insert(user_file)
            .exec_without_returning(&tx)
            .await?;

        if let Some(token_hashes) = validated.search_tokens_hashed {
            if !token_hashes.is_empty() {
                super::multikey_upload::upsert_tokens(&tx, new_file_id, token_hashes).await?;
            }
        }

        audit::append_event(
            &tx,
            NewAuditEvent {
                sender_id: Some(caller.id),
                recipient_id: None,
                file_id: source_file_id,
                action_str: "fork",
                share_role_before: None,
                share_role_after: None,
                created_at: signed_timestamp,
                event_signature: Some(event_signature_b64.clone()),
            },
        )
        .await?;

        tx.commit().await?;

        Ok(ForkOutput {
            new_file_id,
            created_at: signed_timestamp,
        })
    }
}

