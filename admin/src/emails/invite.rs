use context::{Context, SenderContract};
use entity::invitations;
use error::{AppResult, Error};

/// Send an invitation email to the provided email address
pub(crate) async fn send(
    context: &Context,
    invitation: &invitations::Model,
    message: Option<String>,
) -> AppResult<()> {
    let sender = match &context.sender {
        Some(s) => s,
        None => {
            log::warn!("No sender configured, skipping activation email sending");

            return Ok(());
        }
    };

    let content = r#"
    <h1>You have been invited to join the {{app_name}}</h1>
        {{message}}
        {{role}}
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

    let mut link = util::url::generate(&link).ok_or_else(|| {
        log::error!("Invalid link generated: {}", &link);

        Error::InternalError("invalid_link".to_string())
    })?;

    link.query_pairs_mut()
        .append_pair("invitation_id", &invitation.id.to_string())
        .append_pair("email", &invitation.email);

    let app_name = context.config.get_app_name();
    let expires_at = util::datetime::from_timestamp(invitation.expires_at)
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

    if let Some(message) = message {
        template.add_template_var("message", format!("<p>{message}</p>").as_str());
    }

    if let Some(role) = invitation.role.as_deref() {
        template.add_template_var("role", format!("<p>With role: {role}</p>").as_str());
    }

    template.add_template_var("app_name", &app_name);
    template.add_template_var("expires_at", &expires_at);
    template.register_content_template(content.as_str())?;

    sender
        .send(vec![template.to(&invitation.email)?])
        .await
        .map(|_| ())
}
