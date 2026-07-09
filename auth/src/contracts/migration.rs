use chrono::Utc;
use cryptfns::identity::KeyType;
use cryptfns::transition::{Certificate, Signatures};
use entity::{
    key_transitions, user_files, users, ActiveValue, ColumnTrait, EntityTrait, Expr, QueryFilter,
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};
use std::str::FromStr;

use crate::data::opaque::{MigrationComplete, MigrationKey};

use super::repository::Repository;

/// Transition certificates older than this (server clock) are rejected, so a
/// captured migration request cannot be replayed later.
const REPLAY_WINDOW_SECONDS: i64 = 300;

/// One-shot migration of a legacy (RSA + bcrypt) account onto Curve25519 +
/// OPAQUE. The whole switch commits in a single transaction guarded on
/// `security_version = 0`, so a crash or a second device racing can never
/// leave a half-migrated, locked-out account.
#[async_trait::async_trait]
pub(crate) trait Migration
where
    Self: Repository,
{
    /// Every `{file_id, encrypted_key}` the user holds, so the client can
    /// re-wrap each key under its new X25519 key.
    async fn migration_keys(&self, user_id: Uuid) -> AppResult<Vec<MigrationKey>> {
        let rows = user_files::Entity::find()
            .filter(user_files::Column::UserId.eq(user_id))
            .all(self.connection())
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| MigrationKey {
                file_id: row.file_id,
                encrypted_key: row.encrypted_key,
            })
            .collect())
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

        // Re-wrap every key the caller holds. Each update must hit exactly one
        // of the caller's own rows — a file_id they don't hold updates nothing
        // and aborts the migration.
        for rewrapped in &data.rewrapped_keys {
            let updated = user_files::Entity::update_many()
                .col_expr(
                    user_files::Column::EncryptedKey,
                    Expr::value(rewrapped.encrypted_key.clone()),
                )
                .filter(user_files::Column::UserId.eq(user.id))
                .filter(user_files::Column::FileId.eq(rewrapped.file_id))
                .exec(&tx)
                .await?;

            if updated.rows_affected != 1 {
                tx.rollback().await?;
                return Err(Error::BadRequest("rewrapped_key_not_owned".to_string()));
            }
        }

        key_transitions::Entity::insert(key_transitions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user.id),
            old_fingerprint: ActiveValue::Set(user.fingerprint.clone()),
            old_key_spki: ActiveValue::Set(old_key_spki),
            old_key_type: ActiveValue::Set(old_key_type.as_str().to_string()),
            new_fingerprint: ActiveValue::Set(data.new_fingerprint.clone()),
            old_signature: ActiveValue::Set(old_signature),
            new_signature: ActiveValue::Set(new_signature),
            issued_at: ActiveValue::Set(data.transition_issued_at),
            created_at: ActiveValue::Set(now),
        })
        .exec_without_returning(&tx)
        .await?;

        tx.commit().await?;

        self.get_by_id(user.id).await
    }
}
