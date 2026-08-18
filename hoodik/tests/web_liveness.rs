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

/// The compatibility handshake. A marketing version cannot answer "can these
/// two talk to each other", so the server publishes the oldest app it will
/// serve and the one it is built for. The app blocks below the first and
/// nudges below the second.
///
/// Pinned here because the failure mode is silent in both directions: drop
/// these and an old app's search quietly returns nothing instead of saying
/// why, which is what this whole mechanism exists to prevent.
#[actix_web::test]
async fn liveness_advertises_client_compatibility() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context)).await;

    let req = test::TestRequest::get().uri("/api/liveness").to_request();
    let body: serde_json::Value =
        test::read_body_json(test::call_service(&app, req).await).await;

    let minimum = body["minimum_client_version"]
        .as_str()
        .expect("minimum_client_version is published");
    let recommended = body["recommended_client_version"]
        .as_str()
        .expect("recommended_client_version is published");

    // Both must parse as versions the client can compare, or the app cannot
    // act on them.
    for value in [minimum, recommended] {
        assert!(
            value.split('.').count() >= 2
                && value.split('.').all(|p| p.parse::<u32>().is_ok()),
            "{value} is not a comparable version"
        );
    }

    // A recommendation below the minimum would be incoherent: it would nudge
    // clients that are already refused.
    let parse = |v: &str| {
        v.split('.')
            .map(|p| p.parse::<u32>().unwrap())
            .collect::<Vec<_>>()
    };
    assert!(
        parse(recommended) >= parse(minimum),
        "recommended {recommended} must not be below minimum {minimum}"
    );
}
