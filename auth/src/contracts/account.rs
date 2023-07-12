use entity::{users, ActiveValue, Uuid};
use error::{AppResult, Error};

use crate::data::{change_password::ChangePassword, two_factor::Enable};

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

        verify_password(&user, current_password.as_deref())?;

        verify_signature(&user, &new_password, signature.as_deref())?;

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
}

/// Verify the password
fn verify_password(user: &users::Model, password: Option<&str>) -> AppResult<()> {
    if let (Some(password), Some(hashed_password)) = (password, &user.password) {
        if !util::password::verify(password, hashed_password) {
            return Err(Error::Unauthorized("invalid_password".to_string()));
        }
    }

    Ok(())
}

/// Verify the signature
fn verify_signature(user: &users::Model, message: &str, signature: Option<&str>) -> AppResult<()> {
    if let Some(signature) = signature {
        return cryptfns::rsa::public::verify(message, signature, &user.pubkey)
            .map(|_| ())
            .map_err(Error::from);
    }

    Ok(())
}
