use context::{Context, DatabaseConnection, SenderContract};
use entity::{user_actions, users};
use error::AppResult;

use crate::actions::UserActions;

/// Create user action for activation of the email after registration, and send the email
/// use any of the sender configurations provided by the context.
pub(crate) async fn send(context: &Context, user: &users::Model) -> AppResult<()> {
    let sender = match &context.sender {
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

    let action = UserActions::<DatabaseConnection>::new(&context)
        .from_user(&user, "activate-email")
        .await?;

    let link = generate_link(context, &action)?;

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

    sender
        .send(vec![template.to(&action.email)?])
        .await
        .map(|_| ())
}

/// Generate link for email activation
fn generate_link(context: &Context, action: &user_actions::Model) -> AppResult<String> {
    Ok(format!(
        "{}/auth/{}/{}",
        context.config.get_client_url(),
        action.action,
        action.id
    ))
}
