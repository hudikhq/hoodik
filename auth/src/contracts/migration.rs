use chrono::Utc;
use cryptfns::asn1::{audit_event_chain_hash, AuditEventRowV1};
use cryptfns::identity::KeyType;
use cryptfns::transition::{verify_key_rotation_audit, Certificate, Signatures};
use entity::{
    key_transitions, links, migration_rewrap_staging, opaque_ksf, share_events, user_files, users,
    ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, Expr, OnConflict, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait, Uuid,
};
use error::{AppResult, Error};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use crate::data::opaque::{
    MigrationComplete, MigrationKey, MigrationKeys, MigrationLinkKey, RewrapBatch,
};

use super::opaque::CURRENT_OPAQUE_PROTOCOL_VERSION;
use super::repository::Repository;

const AUDIT_HASH_LEN: usize = 32;

/// Transition certificates older than this (server clock) are rejected, so a
/// captured migration request cannot be replayed later.
const REPLAY_WINDOW_SECONDS: i64 = 300;

/// Largest page `migration/keys` will return, and the ceiling on how many keys a
/// single `migration/rewrap` batch may stage. A hybrid X25519+ML-KEM wrap is
/// ~1.7 KB of base64, so 500 keys is under a megabyte — small enough to keep the
/// request body and the server's per-page memory bounded while few enough
/// requests cover even a very large account.
const MIGRATION_KEYS_MAX_PAGE: i64 = 500;
const MAX_REWRAP_ENTRIES_PER_REQUEST: usize = MIGRATION_KEYS_MAX_PAGE as usize;

/// Staging rows older than this are abandoned migrations; a fresh `rewrap`
/// purges them before staging its own, the way `opaque_login_sessions` is purged
/// on login-start. Far longer than a single ceremony's batches take, so an
/// in-progress migration is never reclaimed out from under itself.
const STAGING_TTL_SECONDS: i64 = 24 * 60 * 60;

/// One-shot migration of a legacy (RSA + bcrypt) account onto Curve25519 +
/// OPAQUE. The whole switch commits in a single transaction guarded on
/// `security_version = 0`, so a crash or a second device racing can never
/// leave a half-migrated, locked-out account.
#[async_trait::async_trait]
pub(crate) trait Migration
where
    Self: Repository,
{
    /// One page of the file keys the user holds and the public link keys they
    /// own, so the client can re-wrap each under its new X25519 key. Link keys
    /// are wrapped under the owner's key too; omitting them would strand every
    /// pre-migration link the owner created. Paged because a hybrid-wrapped set
    /// for a large account is many megabytes — this bounds server memory to one
    /// page regardless of account size. The cursor walks file keys first, then
    /// link keys, so `offset` indexes into their concatenation.
    async fn migration_keys(
        &self,
        user_id: Uuid,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> AppResult<MigrationKeys> {
        let offset = offset.unwrap_or(0).max(0);
        let limit = limit.unwrap_or(MIGRATION_KEYS_MAX_PAGE).clamp(1, MIGRATION_KEYS_MAX_PAGE);

        let file_count = user_files::Entity::find()
            .filter(user_files::Column::UserId.eq(user_id))
            .count(self.connection())
            .await? as i64;
        let link_count = links::Entity::find()
            .filter(links::Column::UserId.eq(user_id))
            .count(self.connection())
            .await? as i64;

        let mut keys = Vec::new();
        if offset < file_count {
            let take = (file_count - offset).min(limit);
            keys = user_files::Entity::find()
                .filter(user_files::Column::UserId.eq(user_id))
                .order_by_asc(user_files::Column::Id)
                .offset(offset as u64)
                .limit(take as u64)
                .all(self.connection())
                .await?
                .into_iter()
                .map(|row| MigrationKey {
                    file_id: row.file_id,
                    encrypted_key: row.encrypted_key,
                })
                .collect();
        }

        // A page that ran out of file keys spills into the link keys, which begin
        // at `file_count` in the combined sequence.
        let mut link_keys = Vec::new();
        let remaining = limit - keys.len() as i64;
        if remaining > 0 {
            let link_offset = (offset - file_count).max(0);
            link_keys = links::Entity::find()
                .filter(links::Column::UserId.eq(user_id))
                .order_by_asc(links::Column::Id)
                .offset(link_offset as u64)
                .limit(remaining as u64)
                .all(self.connection())
                .await?
                .into_iter()
                .map(|row| MigrationLinkKey {
                    link_id: row.id,
                    encrypted_link_key: row.encrypted_link_key,
                    file_id: row.file_id,
                })
                .collect();
        }

        let next = offset + limit;
        let next_offset = (next < file_count + link_count).then_some(next);

        Ok(MigrationKeys { keys, link_keys, next_offset })
    }

    /// Stage one batch of re-wrapped keys for a still-legacy account. The client
    /// splits its whole re-wrap into these batches so no single request body has
    /// to carry every hybrid-wrapped key at once; `migration/complete` then
    /// applies the accumulated set atomically. Idempotent on `(user_id, file_id)`
    /// and `(user_id, link_id)`, so a retried batch — normal on a flaky
    /// connection — replaces its rows instead of duplicating them.
    async fn stage_rewrap(&self, user_id: Uuid, batch: RewrapBatch) -> AppResult<()> {
        let user = self.get_by_id(user_id).await?;

        // A migrated account has no legacy keys left to re-wrap.
        if user.security_version != 0 {
            return Err(Error::BadRequest("already_migrated".to_string()));
        }

        if batch.keys.len() + batch.link_keys.len() > MAX_REWRAP_ENTRIES_PER_REQUEST {
            return Err(Error::BadRequest("rewrap_batch_too_large".to_string()));
        }

        // Ownership is checked again when `complete` applies each row, but
        // rejecting a foreign id here fails the client early and keeps another
        // user's ids out of the caller's staging set entirely.
        if !batch.keys.is_empty() {
            let file_ids: Vec<Uuid> = batch.keys.iter().map(|k| k.file_id).collect();
            let owned: HashSet<Uuid> = user_files::Entity::find()
                .filter(user_files::Column::UserId.eq(user.id))
                .filter(user_files::Column::FileId.is_in(file_ids.clone()))
                .all(self.connection())
                .await?
                .into_iter()
                .map(|row| row.file_id)
                .collect();
            if file_ids.iter().any(|id| !owned.contains(id)) {
                return Err(Error::BadRequest("rewrapped_key_not_owned".to_string()));
            }
        }

        if !batch.link_keys.is_empty() {
            let link_ids: Vec<Uuid> = batch.link_keys.iter().map(|k| k.link_id).collect();
            let owned: HashSet<Uuid> = links::Entity::find()
                .filter(links::Column::UserId.eq(user.id))
                .filter(links::Column::Id.is_in(link_ids.clone()))
                .all(self.connection())
                .await?
                .into_iter()
                .map(|row| row.id)
                .collect();
            if link_ids.iter().any(|id| !owned.contains(id)) {
                return Err(Error::BadRequest("rewrapped_link_key_not_owned".to_string()));
            }
        }

        let now = Utc::now().timestamp();

        // Reclaim rows from migrations other users abandoned, on write rather
        // than via a background task — the same purge-on-use pattern the OPAQUE
        // login sessions use.
        migration_rewrap_staging::Entity::delete_many()
            .filter(migration_rewrap_staging::Column::CreatedAt.lt(now - STAGING_TTL_SECONDS))
            .exec(self.connection())
            .await?;

        let tx = self.ctx().db.begin().await?;

        if !batch.keys.is_empty() {
            let rows = batch.keys.iter().map(|k| migration_rewrap_staging::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                user_id: ActiveValue::Set(user.id),
                file_id: ActiveValue::Set(Some(k.file_id)),
                link_id: ActiveValue::Set(None),
                encrypted_key: ActiveValue::Set(k.encrypted_key.clone()),
                signature: ActiveValue::Set(None),
                created_at: ActiveValue::Set(now),
            });
            migration_rewrap_staging::Entity::insert_many(rows)
                .on_conflict(
                    OnConflict::columns([
                        migration_rewrap_staging::Column::UserId,
                        migration_rewrap_staging::Column::FileId,
                    ])
                    .update_columns([
                        migration_rewrap_staging::Column::EncryptedKey,
                        migration_rewrap_staging::Column::CreatedAt,
                    ])
                    .to_owned(),
                )
                .exec_without_returning(&tx)
                .await?;
        }

        if !batch.link_keys.is_empty() {
            let rows = batch.link_keys.iter().map(|k| migration_rewrap_staging::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                user_id: ActiveValue::Set(user.id),
                file_id: ActiveValue::Set(None),
                link_id: ActiveValue::Set(Some(k.link_id)),
                encrypted_key: ActiveValue::Set(k.encrypted_link_key.clone()),
                signature: ActiveValue::Set(Some(k.signature.clone())),
                created_at: ActiveValue::Set(now),
            });
            migration_rewrap_staging::Entity::insert_many(rows)
                .on_conflict(
                    OnConflict::columns([
                        migration_rewrap_staging::Column::UserId,
                        migration_rewrap_staging::Column::LinkId,
                    ])
                    .update_columns([
                        migration_rewrap_staging::Column::EncryptedKey,
                        migration_rewrap_staging::Column::Signature,
                        migration_rewrap_staging::Column::CreatedAt,
                    ])
                    .to_owned(),
                )
                .exec_without_returning(&tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn migration_complete(
        &self,
        user_id: Uuid,
        data: MigrationComplete,
    ) -> AppResult<users::Model> {
        let user = self.get_by_id(user_id).await?;

        if user.security_version != 0 {
            return Err(Error::BadRequest("already_migrated".to_string()));
        }

        let now = Utc::now().timestamp();
        if (now - data.transition_issued_at).abs() > REPLAY_WINDOW_SECONDS {
            return Err(Error::BadRequest("transition_timestamp_skew".to_string()));
        }

        // The envelope is opaque to the server, but an empty one would replace
        // the user's only copy of their private key with nothing — a one-way
        // brick. The client self-check should already prevent this; reject it
        // here regardless.
        if data.encrypted_private_key.trim().is_empty() {
            return Err(Error::BadRequest("encrypted_private_key_required".to_string()));
        }

        // Never trust the wire's fingerprint — recompute it from the key.
        let recomputed = KeyType::Curve25519
            .fingerprint(&data.new_identity_pubkey)
            .map_err(|_| Error::BadRequest("invalid_new_identity_pubkey".to_string()))?;
        if recomputed != data.new_fingerprint {
            return Err(Error::BadRequest("new_fingerprint_mismatch".to_string()));
        }

        let old_key_type = KeyType::from_str(&user.key_type)?;
        let certificate = Certificate {
            user_id: user.id.into_bytes(),
            old_key_type,
            old_key_pem: &user.pubkey,
            old_fingerprint: &user.fingerprint,
            new_identity_key_pem: &data.new_identity_pubkey,
            new_wrapping_key_pem: &data.new_wrapping_pubkey,
            new_fingerprint: &data.new_fingerprint,
            issued_at: data.transition_issued_at,
        };
        certificate
            .verify(&Signatures {
                old_signature: data.transition_old_signature.clone(),
                new_signature: data.transition_new_signature.clone(),
            })
            .map_err(|_| Error::BadRequest("transition_certificate_invalid".to_string()))?;

        // The key-rotation audit signature, re-encoded from server state (user
        // id, old fingerprint from the row, the new fingerprint just verified)
        // and checked against the new identity key. A missing or bad signature
        // aborts before anything is written — the rotation never lands unlogged.
        verify_key_rotation_audit(
            &user.id.into_bytes(),
            &user.fingerprint,
            &data.new_fingerprint,
            data.transition_issued_at,
            &data.audit_event_signature,
            &data.new_identity_pubkey,
        )
        .map_err(|_| Error::BadRequest("audit_event_signature_invalid".to_string()))?;

        let password_file =
            cryptfns::opaque::server_registration_finish(&data.opaque_registration_upload)
                .map_err(|_| Error::BadRequest("opaque_registration_upload_invalid".to_string()))?;

        let old_key_spki = old_key_type
            .member_pubkey_der(&user.pubkey)
            .map_err(|_| Error::InternalError("old_pubkey_der".to_string()))?;
        let old_signature = cryptfns::base64::decode(&data.transition_old_signature)
            .map_err(|_| Error::BadRequest("transition_signature_invalid_base64".to_string()))?;
        let new_signature = cryptfns::base64::decode(&data.transition_new_signature)
            .map_err(|_| Error::BadRequest("transition_signature_invalid_base64".to_string()))?;

        let tx = self.ctx().db.begin().await?;

        // Flip the account only if it is still legacy. A racing migration on
        // another device turns this into a zero-row update.
        let flipped = users::Entity::update_many()
            .col_expr(users::Column::Pubkey, Expr::value(data.new_identity_pubkey.clone()))
            .col_expr(users::Column::Fingerprint, Expr::value(data.new_fingerprint.clone()))
            .col_expr(users::Column::KeyType, Expr::value(KeyType::Curve25519.as_str()))
            .col_expr(
                users::Column::WrappingPubkey,
                Expr::value(data.new_wrapping_pubkey.clone()),
            )
            .col_expr(
                users::Column::EncryptedPrivateKey,
                Expr::value(data.encrypted_private_key.clone()),
            )
            .col_expr(users::Column::OpaquePasswordFile, Expr::value(password_file))
            .col_expr(users::Column::SecurityVersion, Expr::value(1))
            .col_expr(
                users::Column::Password,
                Expr::value(Option::<String>::None),
            )
            .filter(users::Column::Id.eq(user.id))
            .filter(users::Column::SecurityVersion.eq(0))
            .exec(&tx)
            .await?;

        if flipped.rows_affected != 1 {
            tx.rollback().await?;
            return Err(Error::BadRequest("already_migrated".to_string()));
        }

        // Apply the re-wrapped keys the client staged through `migration/rewrap`.
        // Reading and clearing them inside this transaction is what keeps the
        // flip and the whole re-key one atomic unit: a crash here rolls the flip
        // back too, and the staging rows survive for the next login to retry.
        let staged = migration_rewrap_staging::Entity::find()
            .filter(migration_rewrap_staging::Column::UserId.eq(user.id))
            .all(&tx)
            .await?;

        // A link's file_id is taken from the caller's own rows, never the wire,
        // so the re-signature is verified against the canonical a reader will
        // later reconstruct.
        let owned_link_file_ids: HashMap<Uuid, Uuid> = links::Entity::find()
            .filter(links::Column::UserId.eq(user.id))
            .all(&tx)
            .await?
            .into_iter()
            .map(|link| (link.id, link.file_id))
            .collect();

        for row in &staged {
            if let Some(file_id) = row.file_id {
                // Each update must hit exactly one of the caller's own rows — a
                // file they no longer hold updates nothing and aborts.
                let updated = user_files::Entity::update_many()
                    .col_expr(
                        user_files::Column::EncryptedKey,
                        Expr::value(row.encrypted_key.clone()),
                    )
                    .filter(user_files::Column::UserId.eq(user.id))
                    .filter(user_files::Column::FileId.eq(file_id))
                    .exec(&tx)
                    .await?;

                // Ownership was verified when the row was staged, so a 0-row
                // update means the file was deleted or its access revoked between
                // that staged attempt and this completion. Its staged key is moot
                // now, so skip it rather than aborting the whole migration; the
                // staging table is cleared for the user below regardless.
                if updated.rows_affected == 0 {
                    continue;
                }
                if updated.rows_affected != 1 {
                    tx.rollback().await?;
                    return Err(Error::BadRequest("rewrapped_key_not_owned".to_string()));
                }
            } else if let Some(link_id) = row.link_id {
                // A link key is re-wrapped and re-signed so the row converges to
                // the new identity instead of keeping a stale RSA signature that
                // would read as invalid post-migration.
                // As with files, ownership was checked at stage time, so a link
                // missing from the owner's set here was deleted between the staged
                // attempt and completion. Skip its stale row.
                let Some(file_id) = owned_link_file_ids.get(&link_id).copied() else {
                    continue;
                };

                let Some(signature) = row.signature.as_deref() else {
                    tx.rollback().await?;
                    return Err(Error::InternalError("staged_link_missing_signature".to_string()));
                };

                if KeyType::Curve25519
                    .verify(&file_id.to_string(), signature, &data.new_identity_pubkey)
                    .is_err()
                {
                    tx.rollback().await?;
                    return Err(Error::BadRequest("link_signature_invalid".to_string()));
                }

                let updated = links::Entity::update_many()
                    .col_expr(
                        links::Column::EncryptedLinkKey,
                        Expr::value(row.encrypted_key.clone()),
                    )
                    .col_expr(links::Column::Signature, Expr::value(signature.to_string()))
                    .filter(links::Column::UserId.eq(user.id))
                    .filter(links::Column::Id.eq(link_id))
                    .exec(&tx)
                    .await?;

                if updated.rows_affected != 1 {
                    tx.rollback().await?;
                    return Err(Error::BadRequest("rewrapped_link_key_not_owned".to_string()));
                }
            }
        }

        migration_rewrap_staging::Entity::delete_many()
            .filter(migration_rewrap_staging::Column::UserId.eq(user.id))
            .exec(&tx)
            .await?;

        key_transitions::Entity::insert(key_transitions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user.id),
            old_fingerprint: ActiveValue::Set(user.fingerprint.clone()),
            old_key_spki: ActiveValue::Set(old_key_spki),
            old_key_type: ActiveValue::Set(old_key_type.as_str().to_string()),
            new_fingerprint: ActiveValue::Set(data.new_fingerprint.clone()),
            // The keys this transition rotated to, stored verbatim as the account
            // now holds them, so a later chain walk re-encodes this hop's
            // certificate canonical from the row alone — no dependence on the
            // account's live keys, which a further rotation would move on from.
            // These are the exact values the certificate above was verified
            // against.
            new_identity_key_pem: ActiveValue::Set(data.new_identity_pubkey.clone()),
            new_wrapping_key_pem: ActiveValue::Set(data.new_wrapping_pubkey.clone()),
            old_signature: ActiveValue::Set(old_signature),
            new_signature: ActiveValue::Set(new_signature),
            issued_at: ActiveValue::Set(data.transition_issued_at),
            created_at: ActiveValue::Set(now),
        })
        .exec_without_returning(&tx)
        .await?;

        // Record the KSF parameters this migration used. The backfilled default
        // already matches today, but writing them keeps the record honest for a
        // future work-factor raise, when the client's parameters would differ
        // from the column default.
        let ksf = cryptfns::opaque::current_ksf_params();
        opaque_ksf::Entity::update_many()
            .col_expr(opaque_ksf::Column::KsfAlgorithm, Expr::value(ksf.algorithm))
            .col_expr(opaque_ksf::Column::KsfMCost, Expr::value(ksf.m_cost as i32))
            .col_expr(opaque_ksf::Column::KsfTCost, Expr::value(ksf.t_cost as i32))
            .col_expr(opaque_ksf::Column::KsfPCost, Expr::value(ksf.p_cost as i32))
            .col_expr(
                opaque_ksf::Column::OpaqueProtocolVersion,
                Expr::value(CURRENT_OPAQUE_PROTOCOL_VERSION),
            )
            .filter(opaque_ksf::Column::Id.eq(user.id))
            .exec(&tx)
            .await?;

        // The in-chain explanation for the key change: a `key_rotation` event
        // on the owner's audit chain, signed by the new identity, hash-chained
        // exactly like every share event.
        append_key_rotation_event(
            &tx,
            user.id,
            data.transition_issued_at,
            &data.audit_event_signature,
        )
        .await?;

        tx.commit().await?;

        self.get_by_id(user.id).await
    }
}

/// Append the account-level `key_rotation` event to the owner's per-sender audit
/// chain. Mirrors `shares`' `append_event` — the same chain rule via the shared
/// [`audit_event_chain_hash`] — but lives here because `auth` cannot depend on
/// `shares`. The signature was already verified against server-reconstructed
/// state; it is stored verbatim.
async fn append_key_rotation_event<C: ConnectionTrait>(
    db: &C,
    user_id: Uuid,
    rotated_at: i64,
    signature_b64: &str,
) -> AppResult<()> {
    let latest = share_events::Entity::find()
        .filter(share_events::Column::SenderId.eq(user_id))
        .order_by(share_events::Column::CreatedAt, Order::Desc)
        .order_by(share_events::Column::Id, Order::Desc)
        .one(db)
        .await?;

    let prev_hash: [u8; AUDIT_HASH_LEN] = match latest {
        Some(row) => row
            .this_event_hash
            .as_slice()
            .try_into()
            .map_err(|_| Error::InternalError("share_events.this_event_hash bad length".into()))?,
        None => [0u8; AUDIT_HASH_LEN],
    };

    let row = AuditEventRowV1 {
        sender_id: user_id.into_bytes(),
        recipient_id: [0u8; 16],
        file_id: [0u8; 16],
        action: "key_rotation".to_string(),
        share_role: None,
        created_at: rotated_at,
    };
    let this_hash =
        audit_event_chain_hash(&prev_hash, &row).map_err(|e| Error::CryptoError(Box::new(e)))?;

    let signature = cryptfns::base64::decode(signature_b64)
        .map_err(|_| Error::BadRequest("audit_event_signature_invalid".to_string()))?;

    share_events::Entity::insert(share_events::ActiveModel {
        id: ActiveValue::Set(Uuid::now_v7()),
        sender_id: ActiveValue::Set(Some(user_id)),
        recipient_id: ActiveValue::Set(None),
        file_id: ActiveValue::Set(None),
        action: ActiveValue::Set("key_rotation".to_string()),
        share_role_before: ActiveValue::Set(None),
        share_role_after: ActiveValue::Set(None),
        created_at: ActiveValue::Set(rotated_at),
        prev_event_hash: ActiveValue::Set(if prev_hash == [0u8; AUDIT_HASH_LEN] {
            None
        } else {
            Some(prev_hash.to_vec())
        }),
        this_event_hash: ActiveValue::Set(this_hash.to_vec()),
        sender_signature: ActiveValue::Set(Some(signature)),
    })
    .exec_without_returning(db)
    .await?;

    Ok(())
}
