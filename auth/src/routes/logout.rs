use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{auth::Auth, data::authenticated::Authenticated, middleware::verify::Verify};

/// Logout user and perform session destroy
#[route("/api/auth/logout", method = "POST", wrap = "Verify::default()")]
pub(crate) async fn logout(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;
    let auth = Auth::new(&context);

    let authenticated = auth.destroy_session(&authenticated.session).await?;

    let mut response = HttpResponse::NoContent();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, true).await?;
    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.finish())
}
