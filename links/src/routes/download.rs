use actix_web::http::header::ContentEncoding;
use actix_web::{
    route,
    web,
    HttpRequest, HttpResponse,
};
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::prelude::*;

use crate::repository::Repository;

/// Download file from a shareable link (raw ciphertext only).
///
/// This route is not authenticated. The server streams the stored ciphertext
/// without performing any decryption. The caller (browser/app) must obtain
/// the file key via the link metadata + link_key (from URL fragment) and
/// decrypt client-side. This closes the last E2EE exception for public links.
///
/// Request:
///  - Query: chunk: i64 - if omitted, every chunk is streamed back to back
///
/// Response: raw ciphertext bytes (Content-Type and generic disposition)
#[route("/api/links/{link_id}", method = "POST")]
pub(crate) async fn download(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let link_id: Uuid = util::actix::path_var(&req, "link_id")?;
    let repository = Repository::new(&context);
    let chunk = util::actix::query_var::<i64>(&req, "chunk").ok();

    let link = repository.get(link_id).await?;

    if link.is_expired() {
        return Err(Error::Unauthorized("link_expired".to_string()));
    }

    // Count one download per completed transfer, not per chunk request. The
    // client fetches the file as N chunks (`?chunk=i`); only the final index
    // closes a download, and a whole-file request (`chunk` omitted) is a single
    // one. A preview or abandoned transfer that never reaches the last chunk
    // correctly does not count.
    let last_chunk = link
        .file_size
        .map(|size| (size as u64).div_ceil(fs::MAX_CHUNK_SIZE_BYTES).max(1) - 1)
        .unwrap_or(0);
    if chunk.is_none_or(|i| i as u64 == last_chunk) {
        repository.increment_downloads(link.id).await?;
    }

    let fs = Fs::new(&context.config);
    let streamer = if link.file_editable {
        fs.stream_v(&link, link.file_active_version, chunk).await?
    } else {
        fs.stream(&link, chunk).await?
    };

    // The name lives in the link's encrypted metadata; the client decrypts it
    // and renames the saved blob itself, so a generic disposition is enough.
    Ok(HttpResponse::Ok()
        .insert_header(ContentEncoding::Identity)
        .insert_header(("Content-Type", link.file_mime))
        .insert_header((
            "Content-Disposition",
            "attachment; filename=\"download\"",
        ))
        .streaming(streamer.stream()))
}

/// Size + mime for the linked file. The encrypted file name is never resolved
/// server-side — the client decrypts the link metadata with the fragment key
/// and applies the real name itself.
///
/// Response: No Content
#[route("/api/links/{link_id}", method = "HEAD")]
pub(crate) async fn head(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let link_id: Uuid = util::actix::path_var(&req, "link_id")?;
    let repository = Repository::new(&context);

    let link = repository.get(link_id).await?;

    if link.is_expired() {
        return Err(Error::Unauthorized("link_expired".to_string()));
    }

    Ok(HttpResponse::NoContent()
        .insert_header(("Content-Type", link.file_mime))
        .insert_header(("Content-Length", link.file_size.unwrap_or(0).to_string()))
        .insert_header(("Content-Disposition", "attachment; filename=\"download\""))
        .finish())
}
