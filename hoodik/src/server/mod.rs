//! # Application server
//!
//! From this module we define all the application HTTP routes and start the server.
//! This module and its sub-modules will give you a good idea of the application endpoints
//! and endpoint Request and Response structs.

use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    middleware::Logger,
    web, App, HttpServer,
};
use config::ssl::SslConfig as _;
use context::Context;
use error::{AppResult, Error};

pub mod client;
pub mod cors;

pub mod middleware {
    //! # Middleware
    //!
    //! Collection of all the middleware used in the application pulled
    //! from various packages we depend on.
    //!
    //! Currently, there are no middleware used in the application.
}

pub mod routes {
    //! # Routes
    //!
    //! Collection of all the routes used in the application pulled
    //! from various packages we depend on.
    pub use auth::routes as auth_routes;
    pub use storage::routes as storage_routes;

    pub use super::client::client;
}

/// Inject the application modules into the server
fn configure(cfg: &mut web::ServiceConfig) {
    auth::routes::configure(cfg);
    storage::routes::configure(cfg);
}

/// Create the web application and inject all the routes into it
pub fn app(
    context: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        // Set the maximum payload size to 1.1x of a single file chunk
        // we are expecting to be uploaded
        .app_data(web::PayloadConfig::new(
            (fs::MAX_CHUNK_SIZE_BYTES as f32 * 1.1) as usize,
        ))
        .app_data(web::Data::new(context))
        .wrap(cors::setup())
        .configure(configure)
        .route(
            "/api/liveness",
            web::get().to(|| async {
                actix_web::HttpResponse::Ok()
                    .json(serde_json::json!({"METHOD": "GET", "message": "I am alive"}))
            }),
        )
        .route(
            "/api/liveness",
            web::post().to(|| async {
                actix_web::HttpResponse::Ok()
                    .json(serde_json::json!({"METHOD": "POST", "message": "I am alive"}))
            }),
        )
        .route(
            "/api/liveness",
            web::head().to(|| async {
                actix_web::HttpResponse::Ok()
                    .json(serde_json::json!({"METHOD": "HEAD", "message": "I am alive"}))
            }),
        )
        .service(routes::client)
}

/// Start the server
pub async fn engage(context: Context) -> AppResult<()> {
    let bind_address = context.config.get_full_bind_address();
    let config = context.config.build_rustls_config()?;

    HttpServer::new(move || {
        app(context.clone()).wrap(Logger::new(
            "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
        ))
    })
    .bind_rustls(&bind_address, config)?
    .run()
    .await
    .map_err(Error::from)
}
