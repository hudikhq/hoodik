use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::account::Account,
    data::{claims::Claims, two_factor::Enable},
};

/// Enable two factor authentication for the user
///
/// Request: [Enable]
#[route("/api/auth/two-factor", method = "POST")]
pub(crate) async fn enable_two_factor(
    context: web::Data<Context>,
    claims: Claims,
    data: web::Json<Enable>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    auth.enable_two_factor(claims.sub, data.into_inner())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
