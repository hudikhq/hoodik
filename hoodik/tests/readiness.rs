//! `/api/readiness` gate.
//!
//! Unlike `/api/liveness` (process is up), readiness proves the instance can
//! actually serve: it returns 200 only when both the database and the storage
//! backend respond, and 503 otherwise — the signal provisioning and the
//! upgrade health gate watch for.

use actix_web::{http::StatusCode, test};
use hoodik::server;
use serde_json::Value;

#[actix_web::test]
async fn readiness_ok_when_db_and_storage_up() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-readiness-ok".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::get().uri("/api/readiness").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ready");

    context.config.app.cleanup();
}

#[actix_web::test]
async fn readiness_503_when_storage_unreachable() {
    let scratch = "../data-test-readiness-bad";
    let mut context = context::Context::mock_with_data_dir(Some(scratch.to_string())).await;
    // Point storage at a path that does not exist so the storage probe fails
    // while the database stays reachable.
    context.config.app.data_dir = "/no/such/hoodik/data/dir".to_string();
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::get().uri("/api/readiness").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["db"], true);
    assert_eq!(body["storage"], false);

    std::fs::remove_dir_all(scratch).ok();
}
