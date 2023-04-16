use actix_files::NamedFile;
use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use error::{AppResult, Error};

use crate::{contract::StorageProvider, repository::Repository, storage::Storage};

/// Get file content by its id
///
/// Response: [actix_files::NamedFile]
#[route(
    "/api/storage/{file_id}",
    method = "GET",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn download(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id = util::actix::path_var(&req, "file_id")?;

    let file = Repository::new(&context.db)
        .query(&authenticated.user)
        .file(file_id)
        .await?;

    let filename = file
        .get_filename()
        .ok_or(Error::NotFound("file_not_found".to_string()))?;

    let fs_file = Storage::new(&context.config)
        .get(&filename)
        .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;

    let named_file = NamedFile::from_file(fs_file, format!("{filename}.enc"))?;

    Ok(named_file.into_response(&req))
}

/// Get head response for a file this will give all the header
/// information, but no file content.
#[route(
    "/api/storage/{file_id}",
    method = "HEAD",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn head(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id = util::actix::path_var(&req, "file_id")?;

    let file = Repository::new(&context.db)
        .query(&authenticated.user)
        .file(file_id)
        .await?;

    let filename = file
        .get_filename()
        .ok_or(Error::NotFound("file_not_found".to_string()))?;

    let _fs_file = Storage::new(&context.config)
        .get(&filename)
        .map_err(|_| error::Error::NotFound("file_not_found".to_string()))?;

    let filename = format!("{filename}.enc");

    Ok(HttpResponse::NoContent()
        .insert_header(("Content-Type", "application/octet-stream"))
        .insert_header(("Content-Length", file.size.unwrap_or(0)))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", filename),
        ))
        .finish())
}
