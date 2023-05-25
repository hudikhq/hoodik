pub mod create;
pub mod delete;
pub mod download;
pub mod get;
pub mod index;
pub mod update;

/// Register the links routes
/// on to the application server
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(create::create);
    cfg.service(delete::delete);
    cfg.service(download::download);
    cfg.service(get::get);
    cfg.service(download::head);
    cfg.service(index::index);
    cfg.service(update::update);
}
