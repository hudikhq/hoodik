use std::str::FromStr;

use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use cryptfns::cipher::Cipher;
use error::{AppResult, Error};
use settings::factory::Factory;

/// Update the current settings for the platform.
///
/// Request: [settings::data::Data]
///
/// Response: [settings::data::Data]
#[route("/api/admin/settings", method = "PUT")]
pub(crate) async fn update(
    staff: Staff,
    context: web::Data<Context>,
    data: web::Json<settings::data::Data>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let cipher = data.sharing.default_cipher();
    if cipher.is_empty() || Cipher::from_str(cipher).is_err() {
        return Err(Error::as_validation("sharing.default_cipher", "unknown_cipher"));
    }

    context
        .settings
        .update(&context.config, data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(context.settings.inner().await.clone()))
}
