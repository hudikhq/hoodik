use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use error::{AppResult, Error};

use crate::{contract::StorageProvider, repository::Repository, storage::Storage};

/// Get file metadata by its id
///
/// Response: [crate::data::app_file::AppFile]
#[route(
    "/api/storage/{file_id}/metadata",
    method = "GET",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn metadata(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id = util::actix::path_var(&req, "file_id")?;

    let mut file = Repository::new(&context.db)
        .query(&authenticated.user)
        .get(file_id)
        .await?;

    if file.is_dir() && !file.is_owner {
        return Err(Error::NotFound("file_or_dir_not_found".to_string()));
    }

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
