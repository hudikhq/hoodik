//! # Storage routes
//!
//! This module exposes routes for manipulating your files and folders.
//! Also it exposes routes for uploading and downloading files.
//!
//! TODO: This module exposes routes for sharing files with other users
//! on the platform.

pub mod create;
pub mod delete;
pub mod download;
pub mod index;
pub mod metadata;
pub mod name_hash;
pub mod search;
pub mod upload;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create::create);
    cfg.service(delete::delete);
    cfg.service(download::download);
    cfg.service(download::head);
    cfg.service(index::index);
    cfg.service(metadata::metadata);
    cfg.service(name_hash::name_hash);
    cfg.service(search::search);
    cfg.service(upload::upload);
}
