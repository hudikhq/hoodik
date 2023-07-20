use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{auth::Auth, contracts::sessions::Sessions, data::authenticated::Authenticated};

/// Kill single session
#[route("/api/auth/account/kill/{id}", method = "POST")]
pub(crate) async fn kill(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let id = util::actix::path_var::<Uuid>(&req, "id")?;

    if id == authenticated.session.id {
        return Err(error::Error::BadRequest(
            "cannot_kill_current_session".to_string(),
        ));
    }

    let session = auth.get(id, authenticated.user.id).await?;

    auth.destroy(&session).await?;

    let mut response = HttpResponse::NoContent();

    Ok(response.finish())
}
