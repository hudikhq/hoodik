use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::account::Account,
    data::{change_password::ChangePassword, claims::Claims},
};

/// Change the user's password, user needs to be authenticated
/// in order to be able to change the password.
///
/// In case the user has forgotten the password, there is always an
/// option to authenticate with the private key and signature.
///
/// The user that has already authenticated with the private key can provide
/// the signature needed to change the password.
///
/// Request: [crate::data::change_password::ChangePassword]
#[route("/api/auth/account/change-password", method = "POST")]
pub(crate) async fn change_password(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<ChangePassword>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    auth.change_password(claims.sub, data.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}
