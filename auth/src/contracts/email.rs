use context::{DatabaseConnection, SenderContract};
use entity::{user_actions, users};
use error::AppResult;

use crate::actions::UserActions;

use super::ctx::Ctx;

/// Email management
#[async_trait::async_trait]
pub(crate) trait Email
where
    Self: Ctx,
{
    fn has_sender(&self) -> bool {
        self.ctx().sender.is_some()
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
            .for_user(user, "activate-email")
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
