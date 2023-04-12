mod create;
mod index;
mod metadata;
mod upload;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(index::index);
    cfg.service(create::create);
    cfg.service(upload::upload);
    cfg.service(metadata::metadata);
}

pub use create::*;
pub use metadata::*;
pub use upload::*;
