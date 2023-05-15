use actix_web::{route, HttpResponse};
use error::AppResult;

use crate::data::authenticated::Authenticated;

/// If the user is authenticated, return the user data, this is used once the frontend refreshes
///
/// Response: [crate::data::authenticated::Authenticated]
#[route("/api/auth/self", method = "POST")]
pub async fn authenticated_self(authenticated: Authenticated) -> AppResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(authenticated))
}
