use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    data::authenticated::{Authenticated, AuthenticatedJwt},
    jwt,
    middleware::verify::Verify,
};

/// Refresh a session to authenticated user
///
/// Response: [crate::data::authenticated::AuthenticatedJwt]
#[route(
    "/api/auth/refresh",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub(crate) async fn refresh(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;
    let auth = Auth::new(&context);

    crate::middleware::extractor::remove_authenticated_session(&authenticated.session.token).await;

    let authenticated = auth.refresh_session(&authenticated.session).await?;
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}
