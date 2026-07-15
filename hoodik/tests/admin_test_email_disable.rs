//! `MAILER_DISABLE_TEST` hides the admin "send test email" card and rejects its
//! endpoint. The flag is deployment config, flipped on the per-test context so
//! the coverage never depends on process-global env under the parallel runner.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use hoodik::server;
use serde_json::Value;

use crate::shares_common::*;

#[actix_web::test]
async fn disabled_flag_is_advertised_and_endpoint_unavailable() {
    let mut context = context::Context::mock_sqlite().await;
    context.config.app.mailer_disable_test = true;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    // The settings GET advertises the flag so the SPA can hide the card.
    let req = test::TestRequest::get()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["mailer_disable_test"], true);

    // The endpoint refuses too, so a hidden button can't simply be curled
    // around. 503 mirrors the sharing kill switch's disabled-feature response.
    let req = test::TestRequest::post()
        .uri("/api/admin/settings/test-email")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[actix_web::test]
async fn default_leaves_test_email_available() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let req = test::TestRequest::get()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["mailer_disable_test"], false);

    // Not forbidden by the flag: the mock has no mailer, so the endpoint returns
    // its "not configured" 200 rather than the 403 the flag would produce.
    let req = test::TestRequest::post()
        .uri("/api/admin/settings/test-email")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(resp.status(), StatusCode::FORBIDDEN);
}
