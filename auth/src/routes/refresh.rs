use actix_web::{web, HttpRequest, HttpResponse};
use context::Context;
use error::{AppResult, Error};

use crate::{auth::Auth, data::extractor::Extractor};

/// This route behaves same as the [crate::routes::authenticated_self] route,
/// but the claims it requires do not have to be still valid, they can be expired.
/// And on top of that, it also requires the refresh token to be present in the request.
/// Once both of those are present, it will refresh the session and return the new session.
///
/// Response: [crate::data::authenticated::Authenticated]
pub async fn refresh(req: HttpRequest) -> AppResult<HttpResponse> {
    let context = req
        .app_data::<web::Data<Context>>()
        .ok_or_else(|| Error::InternalError("missing_context".to_string()))?;

    let _claims = Extractor::default().jwt(context).req(&req)?;

    let refresh_token = Extractor::default().refresh(context).req(&req)?;

    let auth = Auth::new(context);
    let authenticated = auth.get_by_refresh(refresh_token).await?;
    let authenticated = auth.refresh_session(&authenticated.session).await?;

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!()).await?;

    Ok(HttpResponse::Ok()
        .cookie(jwt)
        .cookie(refresh)
        .json(authenticated))
}
