use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;
use serde::Serialize;

/// The persisted settings plus the runtime capability flags the admin SPA needs
/// to render. `mailer_disable_test` is deployment config, not stored settings,
/// so it rides on the response rather than the persisted [settings::data::Data].
#[derive(Serialize)]
struct SettingsResponse {
    #[serde(flatten)]
    settings: settings::data::Data,
    mailer_disable_test: bool,
}

/// Get the current settings for the platform.
///
/// Response: [SettingsResponse]
#[route("/api/admin/settings", method = "GET")]
pub(crate) async fn index(staff: Staff, context: web::Data<Context>) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let response = SettingsResponse {
        settings: context.settings.inner().await.clone(),
        mailer_disable_test: context.config.app.mailer_disable_test,
    };

    Ok(HttpResponse::Ok().json(response))
}
