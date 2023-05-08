//! # Authentication routes
//!
//! This module is authentication controller for the application
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
    cfg.service(register::register);
    cfg.service(authenticated_self::authenticated_self);
    cfg.service(credentials::credentials);
    cfg.service(logout::logout);
    cfg.service(signature::signature);
    cfg.service(refresh::refresh);
    cfg.service(generate_two_factor::generate_two_factor);
}
