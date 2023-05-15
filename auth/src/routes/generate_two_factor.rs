use actix_web::{route, HttpResponse};
use error::AppResult;

use crate::auth::Auth;

/// Generate a two factor secret for the user
///
/// Response [String]
#[route("/api/auth/two-factor-secret", method = "GET")]
pub async fn generate_two_factor() -> AppResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({ "secret": Auth::generate_two_factor() })))
}
