//! # Storage routes
//!
//! This module exposes routes for manipulating your files and folders.
//! Also it exposes routes for uploading and downloading files.
//!
//! TODO: This module exposes routes for sharing files with other users
//! on the platform.

pub mod create;
pub mod delete;
pub mod delete_many;
pub mod download;
pub mod index;
pub mod metadata;
pub mod move_many;
pub mod name_hash;
pub mod rename;
pub mod search;
pub mod stats;
pub mod upload;

/// Register the storage routes
/// on to the application server
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create::create);
    cfg.service(delete_many::delete_many);
    cfg.service(delete::delete);
    cfg.service(download::download);
    cfg.service(download::head);
    cfg.service(index::index);
    cfg.service(metadata::metadata);
    cfg.service(move_many::move_many);
    cfg.service(name_hash::name_hash);
    cfg.service(rename::rename);
    cfg.service(search::search);
    cfg.service(stats::stats);
    cfg.service(upload::upload);
}
