use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth, contract::AuthProviderContract, data::signature::Signature,
    providers::signature::SignatureProvider,
};

/// Perform user authentication with a key fingerprint and signature
///
/// Request: [crate::data::signature::Signature]
///
/// Response: [crate::data::authenticated::Authenticated]
#[route("/api/auth/signature", method = "POST")]
pub(crate) async fn signature(
    context: web::Data<Context>,
    data: web::Json<Signature>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = SignatureProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;

    let mut response = HttpResponse::Ok();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!()).await?;

    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.json(authenticated))
}
