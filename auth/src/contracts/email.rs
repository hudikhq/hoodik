use context::{DatabaseConnection, SenderContract};
use entity::{user_actions, users};
use error::{AppResult, Error};

use crate::actions::UserActions;

use super::ctx::Ctx;

pub(crate) const ACTION_NAME: &str = "activate-email";
pub(crate) const ACTION_COOLDOWN_IN_MINUTES: i64 = 1;

/// Email management
#[async_trait::async_trait]
pub(crate) trait Email
where
    Self: Ctx,
{
    fn has_sender(&self) -> bool {
        self.ctx().sender.is_some()
    }

    /// Resend activation email to the user, if the cooldown has passed.
    async fn resend_activation(&self, user: &users::Model) -> AppResult<()> {
        if let Ok((user_action, _)) = UserActions::<DatabaseConnection>::new(&self.ctx().db)
            .get_by_email_and_action(&user.email, ACTION_NAME)
            .await
        {
            if user_action.created_at + (ACTION_COOLDOWN_IN_MINUTES * 60)
                > chrono::Utc::now().timestamp()
            {
                return Err(Error::TooManyRequests("too_soon".to_string()));
            }
        }

        self.email_activation(user).await
    }

    /// Send activation email to the user
    async fn email_activation(&self, user: &users::Model) -> AppResult<()> {
        let sender = match &self.ctx().sender {
            Some(s) => s,
            None => {
                log::warn!("No sender configured, skipping activation email sending");

                return Ok(());
            }
        };

        let content = r#"
        <h1>Activate your account</h1>
        <p>
            Please click the link below to activate your account.
        </p>
        <p>
            <a href="{{link}}" class="btn-primary">Activate</a>
        </p>
        <p>
            <a href="{{link}}">{{link}}</a>
        </p>
        "#
        .to_string();

        let action = UserActions::<DatabaseConnection>::new(&self.ctx().db)
            .for_user(user, ACTION_NAME)
            .await?;

        let link = self.generate_client_link(&action)?;

        let mut template = sender.template(
            format!("Account activation token: {}", &action.id).as_str(),
            format!(
                "Click on the provided link to activate your account: {}",
                &link
            )
            .as_str(),
        )?;
        template.add_template_var("link", &link);
        template.register_content_template(content.as_str())?;

        let template = template.to(&action.email)?;

        sender.send(vec![template]).await?;

        Ok(())
    }

    /// Generate link for email activation
    fn generate_client_link(&self, action: &user_actions::Model) -> AppResult<String> {
        Ok(format!(
            "{}/auth/{}/{}",
            self.ctx().config.get_client_url(),
            action.action,
            action.id
        ))
    }
}
