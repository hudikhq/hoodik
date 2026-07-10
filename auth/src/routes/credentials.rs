use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::{cookies::Cookies, provider::AuthProvider},
    data::credentials::Credentials,
    providers::credentials::CredentialsProvider,
};

/// Perform user login with basic credentials
///
/// Request: [crate::data::credentials::Credentials]
///
/// Response: [crate::data::authenticated::Authenticated]
#[route("/api/auth/login", method = "POST")]
pub(crate) async fn credentials(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let (user_agent, ip) = util::actix::extract_ip_ua(&req);

    let identity = data.email.as_deref().unwrap_or_default().trim().to_lowercase();
    let identity = (!identity.is_empty()).then_some(identity.as_str());
    let now = chrono::Utc::now().timestamp();
    crate::rate_limit::check(identity, &ip, now)?;

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let authenticated = match provider.authenticate(&user_agent, &ip).await {
        Ok(authenticated) => authenticated,
        Err(e) => {
            crate::rate_limit::charge_failure(identity, &ip, now);
            return Err(e);
        }
    };

    let mut response = HttpResponse::Ok();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!())?;

    if !context.config.auth.use_headers_for_auth {
        response.cookie(jwt);
        response.cookie(refresh);
    } else {
        response.append_header((
            "x-auth-jwt".to_string(), 
            jwt.value()
        ));
        response.append_header((
            "x-auth-refresh".to_string(), 
            refresh.value()
        ));
    }

    Ok(response.json(authenticated))
}
