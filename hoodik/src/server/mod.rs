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
use context::Context;
use error::{AppResult, Error};

pub mod client;
pub mod cors;

/// Inject the application modules into the server
fn configure(cfg: &mut web::ServiceConfig) {
    admin::routes::configure(cfg);
    auth::routes::configure(cfg);
    links::routes::configure(cfg);
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
        .service(client::client)
}

/// Start the server
pub async fn engage(context: Context) -> AppResult<()> {
    let bind_address = context.config.get_full_bind_address();
    let disabled = context.config.ssl.disabled;
    let app_url = context.config.get_app_url();
    let config = context.config.ssl.build_rustls_config(vec![app_url])?;
    
    let server = HttpServer::new(move || {
        app(context.clone()).wrap(Logger::new(
            "%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
        ))
    });

    if disabled {
        server.bind(&bind_address)?.run().await.map_err(Error::from)
    } else {
        server
        .bind_rustls(&bind_address, config)?
        .run()
        .await
        .map_err(Error::from)
    }
}
