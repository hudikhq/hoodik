use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{auth::Auth, contracts::sessions::Sessions, data::authenticated::Authenticated};

/// Kill all other users sessions
#[route("/api/auth/account/kill-all", method = "POST")]
pub(crate) async fn kill_all(
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    auth.destroy_all(authenticated.session.id, authenticated.user.id)
        .await?;

    let mut response = HttpResponse::NoContent();

    Ok(response.finish())
}
