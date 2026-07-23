//! # Client code module
//!
//! Through this module we are serving all the built client static files.
use crate::client::{_CLIENT, _DEFAULT};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use context::Context;

/// Build output under `assets/` is content-hashed, so a URL's body can never
/// change — cache it for a year. Everything else (icons, manifest) gets a
/// modest window.
const CACHE_HASHED: &str = "public, max-age=31536000, immutable";
const CACHE_STATIC: &str = "public, max-age=3600";

/// The HTML shell must never be cached as immutable: it is the one file
/// whose content changes on deploy while its URL stays the same, and a
/// cached stale shell references hashed chunks that no longer exist.
const CACHE_SHELL: &str = "no-cache";

fn cache_control(filename: &str) -> &'static str {
    if filename.ends_with(".html") {
        CACHE_SHELL
    } else if filename.starts_with("assets/") {
        CACHE_HASHED
    } else {
        CACHE_STATIC
    }
}

/// Get content type from a filename
fn content_type(filename: &str) -> &str {
    match filename.split('.').next_back().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        // The correct type is required for WebAssembly.instantiateStreaming —
        // browsers refuse to stream-compile application/octet-stream.
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// Catch all requests that don't match any internal routes and forward them to the frontend
#[get("/{filename:.*}")]
pub(crate) async fn client(
    _context: web::Data<Context>,
    _req: HttpRequest,
    info: web::Path<String>,
) -> impl Responder {
    let filename = info.into_inner();

    for (path, contents) in _CLIENT {
        if path == filename {
            let content_type = content_type(&filename);

            log::debug!("Client: {} -> {}", filename, content_type);

            return HttpResponse::Ok()
                .insert_header(("Cache-Control", cache_control(&filename)))
                .content_type(content_type)
                .body(contents);
        }
    }

    let path: Vec<&str> = filename.split('.').collect();

    if path.len() == 1 && !filename.starts_with("/api/") {
        return HttpResponse::Ok()
            .insert_header(("Cache-Control", CACHE_SHELL))
            .content_type("text/html; charset=utf-8")
            .body(_DEFAULT);
    }

    log::warn!("Client: Not found: {}", filename);

    HttpResponse::NotFound().finish()
}
