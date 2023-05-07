use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{authenticated::AuthenticatedJwt, signature::Signature},
    jwt,
    providers::signature::SignatureProvider,
};

/// Perform user authentication with a key fingerprint and signature
///
/// Request: [crate::data::signature::Signature]
///
/// Response: [crate::data::authenticated::AuthenticatedJwt]
#[route("/api/auth/signature", method = "POST")]
pub(crate) async fn signature(
    context: web::Data<Context>,
    data: web::Json<Signature>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = SignatureProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}
