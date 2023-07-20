use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{auth::Auth, contracts::account::Account, data::change_password::ChangePassword};

/// Change the users password with the provided current password or private key
///
/// Request: [crate::data::change_password::ChangePassword]
#[route("/api/auth/account/change-password", method = "POST")]
pub(crate) async fn change_password(
    context: web::Data<Context>,
    data: web::Json<ChangePassword>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    auth.change_password(data.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}
