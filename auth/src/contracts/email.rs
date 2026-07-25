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

    /// Send activation email to the user, in the user's preferred language.
    async fn email_activation(&self, user: &users::Model) -> AppResult<()> {
        let sender = match &self.ctx().sender {
            Some(s) => s,
            None => {
                log::warn!("No sender configured, skipping activation email sending");

                return Ok(());
            }
        };

        let copy = activation_copy(util::locale::resolve(user.locale.as_deref()));

        let action = UserActions::<DatabaseConnection>::new(&self.ctx().db)
            .for_user(user, ACTION_NAME)
            .await?;

        let link = self.generate_client_link(&action)?;

        let mut template = sender.template(
            format!("{}{}", copy.subject_prefix, &action.id).as_str(),
            format!("{}{}", copy.pre_header, &link).as_str(),
        )?;
        template.add_template_var("link", &link);
        template.register_content_template(copy.content)?;

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

pub(crate) struct ActivationCopy {
    pub subject_prefix: &'static str,
    pub pre_header: &'static str,
    pub content: &'static str,
}

pub(crate) fn activation_copy(locale: &str) -> ActivationCopy {
    match locale {
        "fr" => ActivationCopy {
            subject_prefix: "Jeton d'activation du compte : ",
            pre_header: "Cliquez sur le lien fourni pour activer votre compte : ",
            content: r#"
        <h1>Activez votre compte</h1>
        <p>
            Veuillez cliquer sur le lien ci-dessous pour activer votre compte.
        </p>
        <p>
            <a href="{{link}}" class="btn-primary">Activer</a>
        </p>
        <p>
            <a href="{{link}}">{{link}}</a>
        </p>
        "#,
        },
        "de" => ActivationCopy {
            subject_prefix: "Kontoaktivierungs-Token: ",
            pre_header: "Klicken Sie auf den bereitgestellten Link, um Ihr Konto zu aktivieren: ",
            content: r#"
        <h1>Konto aktivieren</h1>
        <p>
            Bitte klicken Sie auf den untenstehenden Link, um Ihr Konto zu aktivieren.
        </p>
        <p>
            <a href="{{link}}" class="btn-primary">Aktivieren</a>
        </p>
        <p>
            <a href="{{link}}">{{link}}</a>
        </p>
        "#,
        },
        "hr" => ActivationCopy {
            subject_prefix: "Token za aktivaciju računa: ",
            pre_header: "Kliknite na priloženi link za aktivaciju računa: ",
            content: r#"
        <h1>Aktivirajte svoj račun</h1>
        <p>
            Kliknite na link ispod za aktivaciju računa.
        </p>
        <p>
            <a href="{{link}}" class="btn-primary">Aktiviraj</a>
        </p>
        <p>
            <a href="{{link}}">{{link}}</a>
        </p>
        "#,
        },
        _ => ActivationCopy {
            subject_prefix: "Account activation token: ",
            pre_header: "Click on the provided link to activate your account: ",
            content: r#"
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
        "#,
        },
    }
}
