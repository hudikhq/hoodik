use actix_web::{
    route,
    web::{self, Bytes},
    HttpRequest, HttpResponse,
};
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::prelude::*;

use crate::{data::download::Download, repository::Repository};

/// Map futures download stream so it can decrypt the file while it is being downloaded.
fn map_chunk(chunk: Result<web::Bytes, Error>, file_key: Vec<u8>) -> Result<Bytes, Error> {
    match chunk {
        Ok(chunk) => cryptfns::aes::decrypt(file_key, chunk.to_vec())
            .map_err(|_| Error::Unauthorized("invalid_file_key".to_string()))
            .map(Bytes::from),
        Err(err) => Err(err),
    }
}

/// Download file from a shareable link.
///
/// This route is not authenticated, and anyone
/// with a valid link and file key can download the file.
/// File is decrypted while it is being downloaded in memory.
/// But before downloading starts the signature is verified so it
/// matches the user that supposedly created the link for that file.
///
/// Request: [crate::data::download::Download]
///
/// Response: [actix_web::web::Bytes]
#[route("/api/links/{link_id}", method = "POST")]
pub(crate) async fn download(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Either<web::Json<Download>, web::Form<Download>>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let link_id: Uuid = util::actix::path_var(&req, "link_id")?;
    let repository = Repository::new(&context);
    let link_key = data.into_inner().into_value()?;

    let link = repository.get(link_id).await?;

    if link.is_expired() {
        return Err(Error::Unauthorized("link_expired".to_string()));
    }

    let filename = link.decrypt_name(&link_key)?;
    let file_key = link.file_key(&link_key)?;

    repository.increment_downloads(link.id).await?;

    let streamer = Fs::new(&context.config)
        .stream(&link, None)
        .await?
        .map(move |chunk| map_chunk(chunk, file_key.clone()));

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", link.file_mime))
        .insert_header(("Content-Length", link.file_size.unwrap_or(0)))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        ))
        .streaming(streamer.stream()))
}

/// Get HEAD information about the file, this route does everything as the one
/// above, except it does not download the file, it only returns the headers.
///
/// Request: [crate::data::download::Download]
///
/// Response: No Content
#[route("/api/links/{link_id}", method = "HEAD")]
pub(crate) async fn head(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Either<web::Json<Download>, web::Form<Download>>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let link_id: Uuid = util::actix::path_var(&req, "link_id")?;
    let repository = Repository::new(&context);
    let link_key = data.into_inner().into_value()?;

    let link = repository.get(link_id).await?;

    if link.is_expired() {
        return Err(Error::Unauthorized("link_expired".to_string()));
    }

    let filename = link.decrypt_name(&link_key)?;

    Ok(HttpResponse::NoContent()
        .insert_header(("Content-Type", link.file_mime))
        .insert_header(("Content-Length", link.file_size.unwrap_or(1)))
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        ))
        .finish())
}
