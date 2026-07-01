use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::AppResult;
use fs::prelude::*;

use crate::repository::Repository;

/// Get file metadata by name hash and directory id user can only
/// query his own files this way.
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{name_hash}/name-hash", method = "GET")]
pub(crate) async fn name_hash(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let name_hash = util::actix::path_var::<String>(&req, "name_hash")?;
    let file_id = util::actix::query_var::<Uuid>(&req, "parent_id").ok();

    // Owner-context lookup by name hash — this
    // route is Owner-only to prevent enumeration. `manage(claims.sub)` is
    // already keyed on the caller's owner_id, but we don't apply an
    // explicit `permission()` check here because `by_name` filters
    // `is_owner=true` directly. Non-owner callers see 404.
    let mut file = Repository::new(&context.db)
        .manage(claims.sub)
        .by_name(&name_hash, file_id)
        .await?;

    if file.is_file() && file.finished_upload_at.is_none() {
        file.uploaded_chunks = Some(Fs::new(&context.config).get_uploaded_chunks(&file).await?);
    }

    Ok(HttpResponse::Ok().json(file))
}
