use actix_web::{route, web, HttpResponse};
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
pub(crate) async fn credentials(
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;

    let mut response = HttpResponse::Ok();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!()).await?;

    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.json(authenticated))
}
