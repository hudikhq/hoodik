use chrono::Utc;
use entity::{
    paginated::Paginated, sessions, sort::Sortable, users, ActiveValue, ColumnTrait, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Uuid,
};
use error::{AppResult, Error};
use std::str::FromStr;
use validr::Validation;

use crate::data::{
    activity_query::ActivityQuery, change_password::ChangePassword, two_factor::Enable,
};

use super::repository::Repository;

#[async_trait::async_trait]
pub(crate) trait Account
where
    Self: Repository,
{
    /// Verify the payload and change the users password
    async fn change_password(&self, data: ChangePassword) -> AppResult<users::Model> {
        let (email, new_password, encrypted_private_key, current_password, signature, token) =
            data.into_data()?;

        let user = self.get_by_email(&email).await?;

        if !user.verify_tfa(token) {
            return Err(Error::Unauthorized("invalid_otp_token".to_string()));
        }

        let proved_by_password = verify_password(&user, current_password.as_deref())?;
        let proved_by_signature = verify_signature(&user, &new_password, signature.as_deref())?;

        // Ownership must be affirmatively proven. Without this, a migrated
        // account (whose `password` column is NULL) would pass `verify_password`
        // for any supplied `current_password` because there is no hash to check
        // against — an unauthenticated takeover.
        if !proved_by_password && !proved_by_signature {
            return Err(Error::Unauthorized("ownership_proof_required".to_string()));
        }

        self.update_user(
            user.id,
            users::ActiveModel {
                password: ActiveValue::Set(Some(util::password::hash(&new_password))),
                encrypted_private_key: ActiveValue::Set(Some(encrypted_private_key)),
                ..Default::default()
            },
        )
        .await
    }

    /// Disable the two factor authentication for the user
    async fn disable_two_factor(&self, id: Uuid, token: Option<String>) -> AppResult<()> {
        let user = self.get_by_id(id).await?;

        if !user.verify_tfa(token) {
            return Err(Error::Unauthorized("invalid_otp_token".to_string()));
        }

        self.update_user(
            user.id,
            users::ActiveModel {
                secret: ActiveValue::Set(None),
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }

    /// Enable two factor authentication for the user
    async fn enable_two_factor(&self, id: Uuid, data: Enable) -> AppResult<()> {
        let secret = data.into_value()?;
        let user = self.get_by_id(id).await?;

        if user.secret.is_some() {
            return Err(Error::BadRequest("two_factor_already_enabled".to_string()));
        }

        self.update_user(
            user.id,
            users::ActiveModel {
                secret: ActiveValue::Set(secret),
                ..Default::default()
            },
        )
        .await?;

        Ok(())
    }

    /// Load the paginated list of users activity (sessions)
    async fn activity(&self, parameters: ActivityQuery) -> AppResult<Paginated<sessions::Model>> {
        let parameters = parameters.validate()?;

        let user_id = parameters
            .user_id
            .ok_or_else(|| Error::BadRequest("user_id_is_required".to_string()))?;

        let mut query = sessions::Entity::find().filter(sessions::Column::UserId.eq(user_id));

        if !parameters.with_expired.unwrap_or(false) {
            query = query.filter(sessions::Column::ExpiresAt.gt(Utc::now().timestamp()));
        }

        if let Some(sort) = parameters.sort.as_ref() {
            query = match parameters.order.as_deref() {
                Some("desc") => sort.sort_desc(query),
                _ => sort.sort_asc(query),
            };
        }

        if let Some(search) = parameters.search {
            let maybe_uuid = Uuid::parse_str(search.as_str()).ok();

            if let Some(uuid) = maybe_uuid {
                query = query.filter(sessions::Column::Id.eq(uuid));
            } else {
                query = query.filter(
                    sessions::Column::Ip
                        .contains(search.as_str())
                        .or(sessions::Column::DeviceId.contains(search.as_str()))
                        .or(sessions::Column::UserAgent.contains(search.as_str())),
                );
            }
        }

        let total = query.clone().count(self.connection()).await?;

        query = query.limit(parameters.limit.unwrap_or(15));
        query = query.offset(parameters.offset.unwrap_or(0));

        let sessions = query.all(self.connection()).await?;

        Ok(Paginated::new(sessions, total))
    }
}

/// Verify the current password. Returns `true` only when a password was
/// supplied and matched. A supplied password against an account with no hash
/// (e.g. migrated accounts, `password = NULL`) is an error, not a silent pass.
fn verify_password(user: &users::Model, password: Option<&str>) -> AppResult<bool> {
    match (password, &user.password) {
        (Some(password), Some(hashed_password)) => {
            if util::password::verify(password, hashed_password) {
                Ok(true)
            } else {
                Err(Error::Unauthorized("invalid_password".to_string()))
            }
        }
        (Some(_), None) => Err(Error::Unauthorized("password_not_set".to_string())),
        (None, _) => Ok(false),
    }
}

/// Verify the signature over `message`. Returns `true` only when a signature
/// was supplied and verified against the user's current key.
fn verify_signature(user: &users::Model, message: &str, signature: Option<&str>) -> AppResult<bool> {
    match signature {
        Some(signature) => {
            let key_type = cryptfns::identity::KeyType::from_str(&user.key_type)?;
            key_type.verify(message, signature, &user.pubkey)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::{verify_password, verify_signature};
    use entity::{users, Uuid};

    fn user(password: Option<&str>) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            role: None,
            quota: None,
            email: "u@example.com".to_string(),
            password: password.map(|p| util::password::hash(p)),
            secret: None,
            pubkey: "pubkey".to_string(),
            fingerprint: "fp".to_string(),
            key_type: "rsa".to_string(),
            wrapping_pubkey: None,
            security_version: 0,
            opaque_password_file: None,
            encrypted_private_key: None,
            email_verified_at: None,
            created_at: 0,
            updated_at: 0,
            share_notifications_enabled: true,
        }
    }

    #[test]
    fn migrated_account_rejects_password_proof() {
        // A migrated account has `password = NULL`. Supplying any current
        // password must be rejected, never silently accepted.
        let migrated = user(None);
        assert!(verify_password(&migrated, Some("anything")).is_err());
    }

    #[test]
    fn absent_credentials_do_not_prove_ownership() {
        let u = user(Some("correct horse battery staple"));
        assert_eq!(verify_password(&u, None).unwrap(), false);
        assert_eq!(verify_signature(&u, "msg", None).unwrap(), false);
    }

    #[test]
    fn correct_password_proves_ownership_wrong_password_errors() {
        let u = user(Some("correct horse battery staple"));
        assert_eq!(verify_password(&u, Some("correct horse battery staple")).unwrap(), true);
        assert!(verify_password(&u, Some("wrong")).is_err());
    }
}
