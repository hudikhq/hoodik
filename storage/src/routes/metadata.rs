use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::AppResult;
use fs::prelude::*;
use std::str::FromStr;

use crate::repository::Repository;

/// Get file metadata by its id.
///
/// Open to anyone with a `user_files` row for the file, owner or
/// non-owner. Recipients need this to navigate into a shared folder
/// (their breadcrumbs view fetches the folder's metadata) and to
/// open files shared with them directly.
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}/metadata", method = "GET")]
pub(crate) async fn metadata(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;

    let repository = Repository::new(&context.db);
    let mut file = repository.query(claims.sub).get(file_id).await?;

    if file.is_file() && file.finished_upload_at.is_none() {
        file.uploaded_chunks = Some(Fs::new(&context.config).get_uploaded_chunks(&file).await?);
    }

    let mut wrapped = [file];
    repository.enrich_owner_emails(&mut wrapped).await?;
    repository.enrich_shared_with_counts(&mut wrapped).await?;
    let [file] = wrapped;

    Ok(HttpResponse::Ok().json(file))
}
