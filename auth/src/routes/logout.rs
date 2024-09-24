use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::{cookies::Cookies, sessions::Sessions},
    data::authenticated::Authenticated,
};

/// Logout user and perform session destroy
#[route("/api/auth/logout", method = "POST")]
pub(crate) async fn logout(
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let mut authenticated = auth.destroy(&authenticated.session).await?;

    let mut response = HttpResponse::NoContent();

    let (jwt, refresh) = auth.manage_cookies(&mut authenticated, "logout")?;
    
    if !context.config.auth.use_headers_for_auth {
        response.cookie(jwt);
        response.cookie(refresh);
    }

    Ok(response.finish())
}
