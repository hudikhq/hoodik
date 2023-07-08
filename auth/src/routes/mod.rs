//! # Authentication routes
//!
//! This module is authentication controller for the application

use actix_web::web;

pub mod action;
pub mod authenticated_self;
pub mod change_password;
pub mod credentials;
pub mod generate_two_factor;
pub mod logout;
pub mod refresh;
pub mod register;
pub mod signature;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(action::action);
    cfg.service(authenticated_self::authenticated_self);
    cfg.service(credentials::credentials);
    cfg.service(generate_two_factor::generate_two_factor);
    cfg.service(logout::logout);
    cfg.service(register::register);
    cfg.service(signature::signature);
    cfg.service(change_password::change_password);

    // Refresh is defined this way because we cannot use constant as path in `web::resource` macro
    cfg.service(web::resource(crate::REFRESH_PATH).route(web::post().to(refresh::refresh)));
}
