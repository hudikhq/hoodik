use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::{email::Email, repository::Repository},
    data::resend_activation::ResendActivation,
};

/// Resend the activation email for an account.
/// This route will always return 200 OK, even if the email does not exist,
/// or the account is already verified.
///
/// Request: [ResendActivation]
#[route("/api/auth/resend-activation", method = "POST")]
pub(crate) async fn resend_activation(
    context: web::Data<Context>,
    data: web::Json<ResendActivation>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let email = data.into_inner().into_value()?;

    if let Ok(user) = auth.get_by_email(&email).await {
        if !user.email_verified_at.is_some() {
            auth.resend_activation(&user).await?;
        }
    }

    Ok(HttpResponse::NoContent().finish())
}
