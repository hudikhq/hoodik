use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{authenticated::AuthenticatedJwt, credentials::Credentials},
    jwt,
    providers::credentials::CredentialsProvider,
};

/// Perform user login with basic credentials
///
/// Request: [crate::data::credentials::Credentials]
///
/// Response: [crate::data::authenticated::AuthenticatedJwt]
#[route("/api/auth/login", method = "POST")]
pub(crate) async fn credentials(
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}
