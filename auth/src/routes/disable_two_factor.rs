use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::account::Account,
    data::{claims::Claims, two_factor::Disable},
};

/// Generate a two factor secret for the user
///
/// Response [String]
#[route("/api/auth/two-factor-secret", method = "DELETE")]
pub(crate) async fn disable_two_factor(
    context: web::Data<Context>,
    claims: Claims,
    data: web::Json<Disable>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let token = data.into_inner().into_value()?;

    auth.disable_two_factor(claims.sub, token).await?;

    Ok(HttpResponse::NoContent().finish())
}
