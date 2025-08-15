use actix_web::{web, HttpRequest, HttpResponse};
use context::Context;
use error::{AppResult, Error};

use crate::{
    auth::Auth,
    contracts::{cookies::Cookies, repository::Repository, sessions::Sessions},
    data::extractor::Extractor,
};

/// This route behaves same as the [crate::routes::authenticated_self] route,
/// but the claims it requires do not have to be still valid, they can be expired.
/// And on top of that, it also requires the refresh token to be present in the request.
/// Once both of those are present, it will refresh the session and return the new session.
///
/// Response: [crate::data::authenticated::Authenticated]
pub(crate) async fn refresh(req: HttpRequest) -> AppResult<HttpResponse> {
    let context = req
        .app_data::<web::Data<Context>>()
        .ok_or_else(|| Error::InternalError("missing_context".to_string()))?;

    let _claims = Extractor::default().jwt(context).req(&req)?;

    let refresh_token = Extractor::default().refresh(context).req(&req)?;

    let auth = Auth::new(context);
    let authenticated = auth.get_by_refresh(refresh_token).await?;
    let authenticated = auth.refresh(&authenticated.session).await?;

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!())?;
    let mut response = HttpResponse::Ok();

    if !context.config.auth.use_headers_for_auth {
        response.cookie(jwt);
        response.cookie(refresh);
    } else {
        response.append_header(("x-auth-jwt".to_string(), jwt.value()));
        response.append_header(("x-auth-refresh".to_string(), refresh.value()));
    }

    Ok(response.json(authenticated))
}
