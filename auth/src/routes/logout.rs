use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{auth::Auth, data::authenticated::Authenticated, middleware::verify::Verify};

/// Logout user and perform session destroy
#[route(
    "/api/auth/logout",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub(crate) async fn logout(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;
    let auth = Auth::new(&context);

    crate::middleware::extractor::remove_authenticated_session(&authenticated.session.token).await;

    let authenticated = auth.destroy_session(&authenticated.session).await?;

    let mut response = HttpResponse::NoContent();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, true).await?;
        response.cookie(cookie);
    }

    Ok(response.finish())
}
