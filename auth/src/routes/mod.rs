//! # Authentication routes
//!
//! This module is authentication controller for the application

use crate::middleware::verify::Verify;
use actix_web::{web, HttpResponse};

mod action;
mod authenticated_self;
mod credentials;
mod generate_two_factor;
mod logout;
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

    cfg.service(
        web::resource(crate::REFRESH_PATH)
            .wrap(Verify::new_refresh())
            .route(web::post().to(|| async { HttpResponse::Ok().finish() })),
    );
}
