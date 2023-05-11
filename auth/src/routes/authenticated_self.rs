use actix_web::{route, HttpRequest, HttpResponse};
use error::AppResult;

use crate::{data::authenticated::Authenticated, middleware::verify::Verify};

/// If the user is authenticated, return the user data, this is used once the frontend refreshes
///
/// Response: [crate::data::authenticated::Authenticated]
#[route("/api/auth/self", method = "POST", wrap = "Verify::default()")]
pub(crate) async fn authenticated_self(req: HttpRequest) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;

    Ok(HttpResponse::Ok().json(authenticated))
}
