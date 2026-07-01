use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::AppResult;
use serde_json::json;

use crate::{
    data::multikey_upload::EvictFromFolderBody, repository::Repository, routes::gate,
};

/// `POST /api/storage/{file_id}/evict-from-folder`.
/// Folder-owner-only. The contributor keeps the file in their drive
/// root; the folder linkage is severed.
#[route("/api/storage/{file_id}/evict-from-folder", method = "POST")]
pub(crate) async fn evict_from_folder(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<EvictFromFolderBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let repository = Repository::new(&context);
    let outcome = repository
        .evict_from_folder(&authenticated.user, file_id, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(json!({ "file_id": outcome.file_id })))
}
