//! # Client code module
//!
//! Through this module we are serving all the built client static files.
use crate::client::{_CLIENT, _DEFAULT};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use context::Context;

const CACHE_CONTROL: &str = "public, max-age=3600, immutable";

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
                .insert_header(("Cache-Control", CACHE_CONTROL))
                .content_type(content_type)
                .body(contents);
        }
    }

    let path: Vec<&str> = filename.split('.').collect();

    if path.len() == 1 && !filename.starts_with("/api/") {
        return HttpResponse::Ok()
            .insert_header(("Cache-Control", CACHE_CONTROL))
            .content_type("text/html; charset=utf-8")
            .body(_DEFAULT);
    }

    log::warn!("Client: Not found: {}", filename);

    HttpResponse::NotFound().finish()
}
