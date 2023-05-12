use actix_web::HttpResponse;
use error::AppResult;

use crate::data::authenticated::Authenticated;

/// This route behaves same as the [crate::routes::authenticated_self] route,
/// but it is used to refresh the JWT token because it is used only
/// with a refresh token and not with a JWT token and specific configuration
/// of the middleware.
///
/// Response: [crate::data::authenticated::Authenticated]
pub(crate) async fn refresh(authenticated: Authenticated) -> AppResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(authenticated))
}
