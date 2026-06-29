use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{repository::Repository, routes::gate};

/// `GET /api/shares/{file_id}` — recipient list for a file the caller owns
/// or co-owns.
#[route("/api/shares/{file_id}", method = "GET")]
pub(crate) async fn list(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let repository = Repository::new(&context);
    let shares = repository
        .recipient_list(&authenticated.user, file_id)
        .await?;

    Ok(HttpResponse::Ok().json(shares))
}
