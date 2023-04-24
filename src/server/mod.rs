//! # Application server
//!
//! From this module we define all the application HTTP routes and start the server.
//! This module and its sub-modules will give you a good idea of the application endpoints
//! and endpoint Request and Response structs.

use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, App, HttpServer,
};
use context::Context;

pub mod middleware {
    //! # Middleware
    //!
    //! Collection of all the middleware used in the application pulled
    //! from various packages we depend on.
    pub use ::auth::middleware::{load::Load, verify::Verify};
}

pub mod routes {
    //! # Routes
    //!
    //! Collection of all the routes used in the application pulled
    //! from various packages we depend on.
    pub use auth::routes as auth_routes;
    pub use storage::routes as storage_routes;
}

pub mod cors;
pub mod proxy;

/// Inject the application features into the server
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
    let auth_middleware = middleware::Load::new_with(&context)
        .add_ignore("/api/auth/register".to_string())
        .add_ignore("/api/auth/login".to_string())
        .add_ignore("/api/auth/signature".to_string());

    App::new()
        // Set the maximum payload size to 1.2x of a single file chunk
        // we are expecting to be uploaded
        .app_data(web::PayloadConfig::new(
            (storage::CHUNK_SIZE_BYTES as f32 * 1.2) as usize,
        ))
        .app_data(web::Data::new(context))
        // Authentication load middleware that only sets it up on the app
        .wrap(auth_middleware)
        .wrap(cors::setup())
        .configure(configure)
        .route(
            "/api/liveness",
            web::get().to(|| async { actix_web::HttpResponse::Ok().body("GET: I am alive") }),
        )
        .route(
            "/api/liveness",
            web::post().to(|| async { actix_web::HttpResponse::Ok().body("POST: I am alive") }),
        )
        // Proxy HTTP requests to frontend
        .default_service(web::to(proxy::http))
}

/// Start the server
pub async fn engage(context: Context) -> std::io::Result<()> {
    let bind_address = context.config.get_full_bind_address();

    HttpServer::new(move || app(context.clone()))
        .bind(&bind_address)?
        .run()
        .await
}
