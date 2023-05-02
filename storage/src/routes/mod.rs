mod create;
mod delete;
mod download;
mod index;
mod metadata;
mod name_hash;
mod search;
mod upload;

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
