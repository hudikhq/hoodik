//! # Authentication routes
//!
//! This module is authentication controller for the application

use actix_web::web;

/// A `migration/rewrap` batch is at most 500 hybrid X25519+ML-KEM keys at
/// ~1.7 KB each (~0.85 MB); `migration/complete` is far smaller. 4 MB leaves
/// generous headroom over the largest legitimate batch while staying well below
/// anything that turns the route into a memory-DoS surface. Set explicitly
/// rather than inheriting actix's undocumented 2 MB `Json` default, which a
/// pre-pagination single migration POST silently exceeded above ~9.6k files.
const MIGRATION_JSON_LIMIT_BYTES: usize = 4 * 1024 * 1024;

pub mod account;
pub mod two_factor;

pub mod action;
pub mod authenticated_self;
pub mod credentials;
pub mod logout;
pub mod opaque;
pub mod refresh;
pub mod register;
pub mod register_status;
pub mod resend_activation;
pub mod signature;
pub mod transfer_token;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(account::activity);
    cfg.service(account::change_password);
    cfg.service(account::kill_all);
    cfg.service(account::kill);
    cfg.service(account::patch_me);
    cfg.service(action::action);
    cfg.service(authenticated_self::authenticated_self);
    cfg.service(credentials::credentials);
    cfg.service(logout::logout);
    cfg.service(opaque::register_start);
    cfg.service(opaque::register_finish);
    cfg.service(opaque::signup_register_start);
    cfg.service(opaque::login_start);
    cfg.service(opaque::login_finish);
    cfg.service(opaque::migration_keys);
    // The two migration POSTs carry the re-wrap batches, so give them an explicit
    // per-resource JSON limit instead of the framework default. Set on the
    // resource (not app-wide) so it is not a body-size lever on every other route.
    cfg.service(
        web::resource("/api/auth/migration/rewrap")
            .app_data(web::JsonConfig::default().limit(MIGRATION_JSON_LIMIT_BYTES))
            .route(web::post().to(opaque::migration_rewrap)),
    );
    cfg.service(
        web::resource("/api/auth/migration/complete")
            .app_data(web::JsonConfig::default().limit(MIGRATION_JSON_LIMIT_BYTES))
            .route(web::post().to(opaque::migration_complete)),
    );
    cfg.service(opaque::key_transitions);
    cfg.service(register::register);
    cfg.service(register_status::register_status);
    cfg.service(resend_activation::resend_activation);
    cfg.service(signature::signature);
    cfg.service(two_factor::disable_two_factor);
    cfg.service(two_factor::enable_two_factor);
    cfg.service(two_factor::generate_two_factor);
    cfg.service(transfer_token::create_transfer_token);

    // Refresh is defined this way because we cannot use constant as path in `web::resource` macro
    cfg.service(web::resource(crate::REFRESH_PATH).route(web::post().to(refresh::refresh)));
}
