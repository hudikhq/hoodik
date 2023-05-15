use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};

use crate::{
    contract::StorageProvider,
    repository::{cached::get_file, Repository},
    storage::Storage,
};

/// Get file content by its id
///
/// Request:
///  - Query: chunk: i32 - if omitted, file be streamed until its completely downloaded
///
/// Response: [actix_web::web::Bytes]
///  - Content-Type: application/octet-stream
#[route("/api/storage/{file_id}", method = "GET")]
pub async fn download(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let chunk = util::actix::query_var::<i32>(&req, "chunk").ok();

    let file = get_file(&context, claims.sub, file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    let filename = file
        .get_filename()
        .ok_or(Error::NotFound("file_not_found".to_string()))?;

    let storage = Storage::new(&context.config);

    let streamer = storage.stream(&filename, chunk).await;

    let filename = match chunk {
        Some(chunk) => format!("{filename}.part.{chunk}.enc"),
        None => format!("{filename}.enc"),
    };

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .insert_header(("Content-Length", file.size.unwrap_or(0)))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        ))
        .streaming(streamer.stream()))
}

/// Get head response for a file this will give all the header
/// information, but no file content.
#[route("/api/storage/{file_id}", method = "HEAD")]
pub async fn head(
    req: HttpRequest,
    context: web::Data<Context>,
    claims: web::Data<Claims>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let chunk = util::actix::query_var::<i32>(&req, "chunk").ok();

    let file = Repository::new(&context.db)
        .query(claims.sub)
        .file(file_id)
        .await?;

    let filename = file
        .get_filename()
        .ok_or(Error::NotFound("file_not_found".to_string()))?;

    let storage = Storage::new(&context.config);

    if let Some(c) = chunk {
        let _fs_file = storage
            .get(&filename, c)
            .await
            .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;
    } else {
        let chunks = storage
            .get_uploaded_chunks(&filename)
            .await
            .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;

        // check that all the chunks are available
        for chunk in chunks {
            let _fs_file = storage
                .get(&filename, chunk)
                .await
                .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;
        }
    }

    let filename = match chunk {
        Some(chunk) => format!("{filename}.part.{chunk}.enc"),
        None => format!("{filename}.enc"),
    };

    Ok(HttpResponse::NoContent()
        .insert_header(("Content-Type", "application/octet-stream"))
        .insert_header(("Content-Length", file.size.unwrap_or(0)))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        ))
        .finish())
}
