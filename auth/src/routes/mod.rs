//! # Authentication routes
//!
//! This module is authentication controller for the application

use crate::middleware::refresh::Refresh;
use actix_web::web;

mod action;
mod authenticated_self;
mod credentials;
mod generate_two_factor;
mod logout;
mod refresh;
mod register;
mod signature;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(action::action);
    cfg.service(authenticated_self::authenticated_self);
    cfg.service(credentials::credentials);
    cfg.service(generate_two_factor::generate_two_factor);
    cfg.service(logout::logout);
    cfg.service(register::register);
    cfg.service(signature::signature);

    // Refresh is defined this way because we cannot use constant as
    // path in `web::resource` macro
    cfg.service(
        web::resource(crate::REFRESH_PATH)
            .wrap(Refresh::default())
            .route(web::post().to(refresh::refresh)),
    );
}
