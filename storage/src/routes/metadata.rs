use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::prelude::*;
use std::str::FromStr;

use crate::repository::Repository;

/// Get file metadata by its id
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

    let mut file = Repository::new(&context.db)
        .query(claims.sub)
        .get(file_id)
        .await?;

    if file.is_dir() && !file.is_owner {
        return Err(Error::NotFound("file_or_dir_not_found".to_string()));
    }

    if file.is_file() && file.finished_upload_at.is_none() {
        let filename = file.get_filename().unwrap();
        file.uploaded_chunks = Some(
            Fs::new(&context.config)
                .get_uploaded_chunks(&filename)
                .await?,
        );
    }

    Ok(HttpResponse::Ok().json(file))
}
