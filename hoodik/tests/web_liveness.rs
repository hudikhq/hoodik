#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use hoodik::server;

/// Clients (Flutter app, Playwright e2e, any monitor) depend on the shape
/// of `/api/liveness` to decide whether a server is reachable AND whether
/// it's recent enough to use new features. This test pins both halves so
/// a well-intentioned cleanup of the inline JSON response can't silently
/// drop the `version` field and regress the app's "server is outdated"
/// warning.
#[actix_web::test]
async fn liveness_get_returns_version_and_legacy_fields() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context)).await;

    let req = test::TestRequest::get().uri("/api/liveness").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["METHOD"], "GET");
    assert_eq!(body["message"], "I am alive");

    let version = body["version"]
        .as_str()
        .expect("liveness response must carry a string `version` field");
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    // Sanity: Cargo.toml enforces semver, but double-check at least one dot
    // so a typo like "1-14-1" or an empty string can't sneak through.
    assert!(
        version.chars().filter(|c| *c == '.').count() >= 2,
        "version '{version}' does not look like semver"
    );
}

#[actix_web::test]
async fn liveness_post_returns_version() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context)).await;

    let req = test::TestRequest::post().uri("/api/liveness").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["METHOD"], "POST");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}
