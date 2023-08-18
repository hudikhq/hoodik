use async_trait::async_trait;
use chrono::Utc;
use entity::{users, ActiveModelTrait, ActiveValue, TransactionTrait, Uuid};
use error::{AppResult, Error};

use crate::{actions::UserActions, data::create_user::CreateUser};

use super::{email::Email, repository::Repository};

/// Register contract implementing all the methods needed for registration
/// and activation of the new users
#[async_trait]
pub(crate) trait Register
where
    Self: Repository + Email,
{
    /// Generate a new 2FA secret
    fn generate_two_factor() -> String {
        util::generate::generate_secret()
    }

    /// Create a new user
    async fn register(&self, data: CreateUser) -> AppResult<users::Model> {
        let email = data.email.clone().unwrap();
        let invitation_id = data.invitation_id;

        let mut active_model = data.into_active_model()?;

        // We can unwrap here because it would fail validation before this
        if self.get_by_email(&email).await.is_ok() {
            return Err(Error::as_validation("email", "invalid_email"));
        }

        if !self.has_sender() {
            active_model.email_verified_at = ActiveValue::Set(Some(Utc::now().timestamp()));
        }

        if let Some(id) = invitation_id {
            let invitation = self.get_invitation(id).await?;

            if invitation.email != email {
                return Err(Error::as_validation("invitation_id", "invalid_invitation"));
            }

            active_model.role = ActiveValue::Set(invitation.role);
            active_model.quota = ActiveValue::Set(invitation.quota);
        } else if self.count_users().await? == 0 {
            active_model.role = ActiveValue::Set(Some("admin".to_string()));
        }

        let user = self.create_user(active_model).await?;

        self.email_activation(&user).await?;

        Ok(user)
    }

    /// Perform activation of the user
    async fn activate(&self, user_action_id: Uuid) -> AppResult<users::Model> {
        let tx = self.connection().begin().await?;

        let user_action = UserActions::new(&tx);

        let (action, user) = user_action.get_by_id(user_action_id).await?;

        if action.action != "activate-email" {
            return Err(Error::as_not_found("wrong_user_action"));
        }

        if user.email_verified_at.is_some() {
            return Err(Error::as_not_found("email_already_verified"));
        }

        let id = user.id;

        let mut active_model: users::ActiveModel = user.into();

        active_model.email_verified_at = ActiveValue::Set(Some(Utc::now().timestamp()));

        active_model.update(&tx).await?;

        user_action.delete(user_action_id).await?;

        tx.commit().await?;

        self.get_by_id(id).await
    }
}
