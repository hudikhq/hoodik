//! # Authentication routes
//!
//! This module is authentication controller for the application

use actix_web::web;

pub mod account;
pub mod two_factor;

pub mod action;
pub mod authenticated_self;
pub mod credentials;
pub mod logout;
pub mod refresh;
pub mod register;
pub mod resend_activation;
pub mod signature;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(account::activity);
    cfg.service(account::change_password);
    cfg.service(account::kill_all);
    cfg.service(account::kill);
    cfg.service(action::action);
    cfg.service(authenticated_self::authenticated_self);
    cfg.service(credentials::credentials);
    cfg.service(logout::logout);
    cfg.service(register::register);
    cfg.service(resend_activation::resend_activation);
    cfg.service(signature::signature);
    cfg.service(two_factor::disable_two_factor);
    cfg.service(two_factor::enable_two_factor);
    cfg.service(two_factor::generate_two_factor);

    // Refresh is defined this way because we cannot use constant as path in `web::resource` macro
    cfg.service(web::resource(crate::REFRESH_PATH).route(web::post().to(refresh::refresh)));
}
