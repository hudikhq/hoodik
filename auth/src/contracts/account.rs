use entity::{users, ActiveValue, Uuid};
use error::{AppResult, Error};

use crate::data::change_password::ChangePassword;

use super::repository::Repository;

#[async_trait::async_trait]
pub(crate) trait Account
where
    Self: Repository,
{
    /// Verify the payload and change the users password
    async fn change_password(
        &self,
        user_id: Uuid,
        data: ChangePassword,
    ) -> AppResult<users::Model> {
        let (new_password, encrypted_private_key, current_password, signature, token) =
            data.into_data()?;

        let user = self.get_by_id(user_id).await?;

        if !user.verify_tfa(token) {
            return Err(Error::Unauthorized("invalid_otp_token".to_string()));
        }

        let mut is_valid = false;

        if let Some(password) = current_password.as_deref() {
            is_valid = verify_password(&user, password);
        }

        if let Some(signature) = signature.as_deref() {
            is_valid = is_valid || verify_signature(&user, &new_password, signature);
        }

        if !is_valid {
            return Err(Error::Unauthorized(
                "invalid_password_or_signature".to_string(),
            ));
        }

        self.update_user(
            user_id,
            users::ActiveModel {
                password: ActiveValue::Set(Some(util::password::hash(&new_password))),
                encrypted_private_key: ActiveValue::Set(Some(encrypted_private_key)),
                ..Default::default()
            },
        )
        .await
    }
}

/// Verify the password
fn verify_password(user: &users::Model, password: &str) -> bool {
    if let Some(hashed_password) = &user.password {
        util::password::verify(password, hashed_password)
    } else {
        false
    }
}

/// Verify the signature
fn verify_signature(user: &users::Model, message: &str, signature: &str) -> bool {
    cryptfns::rsa::public::verify(message, signature, &user.pubkey).is_ok()
}
