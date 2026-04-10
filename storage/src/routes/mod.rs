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
pub mod replace_content;
pub mod search;
pub mod set_editable;
pub mod stats;
pub mod update_hashes;
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
    cfg.service(replace_content::replace_content);
    cfg.service(search::search);
    cfg.service(set_editable::set_editable);
    cfg.service(stats::stats);
    cfg.service(update_hashes::update_hashes);
    cfg.service(upload::upload);
}
