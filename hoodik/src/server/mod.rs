//! # Application server
//!
//! From this module we define all the application HTTP routes and start the server.
//! This module and its sub-modules will give you a good idea of the application endpoints
//! and endpoint Request and Response structs.

use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    middleware::Logger,
    web, App, HttpServer,
};
use context::Context;
use error::{AppResult, Error};
use fs::prelude::{Fs, FsProviderContract};

pub mod client;
pub mod cors;

/// Inject the application modules into the server
fn configure(cfg: &mut web::ServiceConfig) {
    admin::routes::configure(cfg);
    auth::routes::configure(cfg);
    links::routes::configure(cfg);
    shares::routes::configure(cfg);
    storage::routes::configure(cfg);
}

/// Create the web application and inject all the routes into it
pub fn app(
    context: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
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
        // Compresses the SPA assets and JSON responses when the client
        // asks for it — the wasm bundle alone drops from 3.7 MB to under
        // half of that, which decides whether a slow connection boots in
        // seconds or minutes. The ciphertext streams opt out with an
        // identity encoding: encrypted bytes don't compress, and the CPU
        // spent trying would tax the hottest path in the server.
        .wrap(actix_web::middleware::Compress::default())
        .wrap(cors::setup())
        .configure(configure)
        .route("/api/liveness", web::get().to(|| liveness("GET")))
        .route("/api/liveness", web::post().to(|| liveness("POST")))
        .route("/api/liveness", web::head().to(|| liveness("HEAD")))
        .route("/api/readiness", web::get().to(readiness))
        .service(client::client)
}

/// Compiled-in server version, surfaced on `/api/liveness` so clients can
/// detect when they're talking to an out-of-date self-hosted instance and
/// nudge the operator to upgrade. Sourced from the crate's Cargo.toml at
/// build time — no runtime config, nothing to misconfigure.
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Oldest app this server can serve at all. Declared in
/// `[package.metadata.compat]` and lifted in by `build.rs`, so a release bumps
/// it next to the version it ships with.
///
/// Below this a client is refused rather than nudged: 2.5.0 moved the search
/// index to tags keyed on material the server never sees, and an older app
/// can only produce the reversible digests this server no longer stores. Its
/// search would return an empty list forever with nothing to explain it.
///
/// Advertised rather than enforced here — the routes that genuinely cannot
/// serve an old client refuse on their own (see `Error::UpgradeRequired`).
/// Publishing it lets the app say so in its own language, at the moment the
/// user tries the thing that would fail.
const MINIMUM_CLIENT_VERSION: &str = env!("HOODIK_MINIMUM_CLIENT_VERSION");

/// App version this server is built to work with.
///
/// Between this and [`MINIMUM_CLIENT_VERSION`] everything still works, so a
/// client below it gets a nudge and nothing more. Raising this is cheap;
/// raising the minimum breaks people, so they are deliberately separate
/// numbers rather than one.
const RECOMMENDED_CLIENT_VERSION: &str = env!("HOODIK_RECOMMENDED_CLIENT_VERSION");

async fn liveness(method: &'static str) -> actix_web::HttpResponse {
    // `METHOD` and `message` are kept for backward compatibility: existing
    // monitors that parse those fields pre-date the version addition, and
    // we don't want a quiet alert flip when operators upgrade.
    actix_web::HttpResponse::Ok().json(serde_json::json!({
        "METHOD": method,
        "message": "I am alive",
        "version": SERVER_VERSION,
        "minimum_client_version": MINIMUM_CLIENT_VERSION,
        "recommended_client_version": RECOMMENDED_CLIENT_VERSION,
    }))
}

/// Readiness gate, distinct from `/api/liveness`: it proves the instance can
/// actually serve traffic, not merely that the process is up. Returns 200 only
/// when both the database and the storage backend respond, otherwise 503 — so
/// a bad S3 credential or a missing data directory fails here rather than at
/// the first upload. Used by provisioning and the upgrade health gate.
async fn readiness(context: web::Data<Context>) -> actix_web::HttpResponse {
    let db_ok = context.db.ping().await.is_ok();
    let storage_ok = Fs::new(&context.config).health_check().await.is_ok();
    let direct = config::direct::verdict();

    if db_ok && storage_ok {
        actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "ready",
            // Reported even when ready: a bucket that cannot serve clients
            // directly is a working deployment, just not the one the
            // operator asked for, and the reasons are the only place they
            // will see why.
            "direct_transfer": direct.enabled,
            "direct_transfer_blockers": direct.blockers,
        }))
    } else {
        actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "not_ready",
            "db": db_ok,
            "storage": storage_ok,
        }))
    }
}

/// Start the server
pub async fn engage(context: Context) -> AppResult<()> {
    // Settle direct transfer before anything can ask about it. It reaches the
    // storage bucket over the network, so it happens once here rather than
    // per request, and a bucket that answers badly costs a log line instead
    // of the boot.
    fs::direct::probe(&context.config).await;

    let bind_address = context.config.get_full_bind_address();
    let disabled = context.config.ssl.disabled;
    let app_url = context.config.get_app_url();
    let workers = context.config.app.workers;
    let rustls_config = if disabled {
        None
    } else {
        Some(context.config.ssl.build_rustls_config(vec![app_url])?)
    };

    let mut server = HttpServer::new(move || {
        // Health probes hit /api/liveness every few seconds; logging them buries
        // every real request, so keep them out of the access log.
        app(context.clone()).wrap(
            Logger::new(
                "%a %{X-Forwarded-For}i \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T",
            )
            .exclude("/api/liveness"),
        )
    });

    if let Some(workers) = workers {
        server = server.workers(workers);
    }

    if disabled {
        server.bind(&bind_address)?.run().await.map_err(Error::from)
    } else {
        let config = rustls_config.expect("rustls config must be built when SSL is enabled");
        server
            .bind_rustls(&bind_address, config)?
            .run()
            .await
            .map_err(Error::from)
    }
}
