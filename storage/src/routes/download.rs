use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};

use crate::{contract::StorageProvider, repository::Repository, storage::Storage};

/// Get file content by its id
///
/// Request:
///  - Query: chunk: i32 - if omitted, first chunk will be downloaded
///
/// Response: [actix_files::NamedFile]
///  - Content-Type: application/octet-stream
///  - File Name will be the original file name
#[route(
    "/api/storage/{file_id}",
    method = "GET",
    wrap = "Verify::csrf_header_default()"
)]
pub(crate) async fn download(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let chunk = util::actix::query_var::<i32>(&req, "chunk").ok();

    let file = Repository::new(&context.db)
        .query(&authenticated.user)
        .file(file_id)
        .await?;

    let filename = file
        .get_filename()
        .ok_or(Error::NotFound("file_not_found".to_string()))?;

    let storage = Storage::new(&context.config);

    let mut files = vec![];

    if let Some(c) = chunk {
        let fs_file = storage
            .get(&filename, c)
            .await
            .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;

        files.push(fs_file);
    } else {
        let chunks = storage
            .get_uploaded_chunks(&filename)
            .await
            .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;

        // check that all the chunks are available
        for chunk in chunks {
            let fs_file = storage
                .get(&filename, chunk)
                .await
                .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;

            files.push(fs_file);
        }
    }

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
#[route(
    "/api/storage/{file_id}",
    method = "HEAD",
    wrap = "Verify::csrf_header_default()"
)]
pub(crate) async fn head(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let chunk = util::actix::query_var::<i32>(&req, "chunk").ok();

    let file = Repository::new(&context.db)
        .query(&authenticated.user)
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
