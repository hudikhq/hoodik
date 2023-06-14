use context::{Context, SenderContract};
use entity::invitations;
use error::AppResult;

/// Send an invitation email to the provided email address
pub(crate) async fn send(
    context: &Context,
    invitation: &invitations::Model,
    message: String,
) -> AppResult<()> {
    let sender = match &context.sender {
        Some(s) => s,
        None => {
            log::warn!("No sender configured, skipping activation email sending");

            return Ok(());
        }
    };

    let content = r#"
    <h1>Join the {{app_name}}</h1>
    <p>
        {{message}}
    </p>
    <p>
        This invitation is valid until: {{expires_at}}
    </p>
    <p>
        <a href="{{link}}" class="btn-primary">Register</a>
    </p>
    <p>
        <a href="{{link}}">{{link}}</a>
    </p>
    "#
    .to_string();

    let link = format!("{}/auth/register", context.config.get_client_url());
    let app_name = context.config.get_app_name();
    let expires_at = invitation
        .expires_at
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let mut template = sender.template(
        "Invitation to register",
        format!(
            "Click on the provided link to create a new account: {}",
            &link
        )
        .as_str(),
    )?;

    template.add_template_var("link", &link);
    template.add_template_var("message", &message);
    template.add_template_var("app_name", &app_name);
    template.add_template_var("expires_at", &expires_at);
    template.register_content_template(content.as_str())?;

    sender
        .send(vec![template.to(&invitation.email)?])
        .await
        .map(|_| ())
}
