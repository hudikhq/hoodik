use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::{Context, SenderContract};
use error::AppResult;

use crate::repository::Repository;

/// Send a test email to the authenticated admin user
///
/// Response: Success message
#[route("/api/admin/settings/test-email", method = "POST")]
pub(crate) async fn test_email(
    staff: Staff,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let sender = match &context.sender {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Email is not configured on this server"
            })));
        }
    };

    // Get the user's email from the database
    let user = Repository::new(&context, &context.db)
        .users()
        .get(staff.claims.sub)
        .await?;

    let content = r#"
    <h1>Test Email from {{app_name}}</h1>
    <p>
        This is a test email to verify your SMTP configuration is working correctly.
    </p>
    <p>
        If you received this email, your email settings are configured properly!
    </p>
    <p>
        <strong>Configuration details:</strong>
    </p>
    <ul>
        <li>Application: {{app_name}}</li>
        <li>Version: {{app_version}}</li>
        <li>Sent at: {{sent_at}}</li>
    </ul>
    "#
    .to_string();

    let app_name = context.config.get_app_name();
    let app_version = context.config.get_app_version();
    let sent_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

    let mut template = sender.template(
        "Test Email - SMTP Configuration",
        "This is a test email to verify your SMTP configuration",
    )?;

    template.add_template_var("app_name", &app_name);
    template.add_template_var("app_version", app_version);
    template.add_template_var("sent_at", &sent_at);
    template.register_content_template(content.as_str())?;

    let template = template.to(&user.email)?;

    sender.send(vec![template]).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": format!("Test email sent successfully to {}", &user.email)
    })))
}

