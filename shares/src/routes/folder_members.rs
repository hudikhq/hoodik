use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{repository::Repository, routes::gate};

/// `GET /api/shares/folder/{folder_id}/members`.
/// Any current member of the folder share can read; non-members receive
/// 404 so the folder's existence doesn't leak.
#[route("/api/shares/folder/{folder_id}/members", method = "GET")]
pub(crate) async fn folder_members(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let folder_id: Uuid = util::actix::path_var(&req, "folder_id")?;
    let repository = Repository::new(&context);
    let response = repository
        .folder_members(&authenticated.user, folder_id)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}
