use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;

/// Get the current settings for the platform.
///
/// Response: [settings::data::Data]
#[route("/api/admin/settings", method = "GET")]
pub(crate) async fn index(staff: Staff, context: web::Data<Context>) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    Ok(HttpResponse::Ok().json(context.settings.inner().await.clone()))
}
