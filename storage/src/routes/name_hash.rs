use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;

use crate::{contract::StorageProvider, repository::Repository, storage::Storage};

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
    let file_id = util::actix::query_var::<i32>(&req, "parent_id").ok();

    let mut file = Repository::new(&context.db)
        .manage(claims.sub)
        .by_name(&name_hash, file_id)
        .await?;

    if file.is_file() && file.finished_upload_at.is_none() {
        let filename = file.get_filename().unwrap();
        file.uploaded_chunks = Some(
            Storage::new(&context.config)
                .get_uploaded_chunks(&filename)
                .await?,
        );
    }

    Ok(HttpResponse::Ok().json(file))
}
