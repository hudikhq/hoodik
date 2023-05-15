use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth, contract::AuthProviderContract, data::credentials::Credentials,
    providers::credentials::CredentialsProvider,
};

/// Perform user login with basic credentials
///
/// Request: [crate::data::credentials::Credentials]
///
/// Response: [crate::data::authenticated::Authenticated]
#[route("/api/auth/login", method = "POST")]
pub async fn credentials(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let (user_agent, ip) = util::actix::extract_ip_ua(&req);

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate(&user_agent, &ip).await?;

    let mut response = HttpResponse::Ok();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!()).await?;

    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.json(authenticated))
}
