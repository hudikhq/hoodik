use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::AppResult;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::repository::Repository;

#[derive(Debug, Serialize, Deserialize)]
pub struct ThumbnailResponse {
    /// Thumbnail encrypted with the file key, `None` when the file has none.
    pub encrypted_thumbnail: Option<String>,
}

/// Get a file's encrypted thumbnail.
///
/// Directory listings and search results only carry a `has_thumbnail`
/// flag; clients fetch the blob here one file at a time, after the rows
/// have already rendered. Open to anyone with a `user_files` row for the
/// file, same as the metadata route.
///
/// Response: [ThumbnailResponse]
#[route("/api/storage/{file_id}/thumbnail", method = "GET")]
pub(crate) async fn thumbnail(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;

    let file = Repository::new(&context.db)
        .query(claims.sub)
        .get(file_id)
        .await?;

    Ok(HttpResponse::Ok().json(ThumbnailResponse {
        encrypted_thumbnail: file.encrypted_thumbnail,
    }))
}
