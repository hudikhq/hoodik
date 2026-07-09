//! Default-cipher advertisement tests. Verifies that `GET /api/capabilities`
//! carries `Settings.sharing.default_cipher`, that an admin can change it
//! through the existing settings PUT, and that unknown ciphers are rejected.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use hoodik::server;
use serde_json::Value;
use shares::data::capabilities::Capabilities;

#[actix_web::test]
async fn test_capabilities_advertises_default_cipher() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let caps: Capabilities = test::read_body_json(resp).await;
    assert_eq!(caps.default_cipher, "aegis128l");
}

#[actix_web::test]
async fn test_admin_settings_updates_default_cipher() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let req = test::TestRequest::get()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let mut payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["sharing"]["default_cipher"], "aegis128l");

    payload["sharing"]["default_cipher"] = Value::String("aegis256".to_string());
    let req = test::TestRequest::put()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let echoed: Value = test::read_body_json(resp).await;
    assert_eq!(echoed["sharing"]["default_cipher"], "aegis256");

    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let caps: Capabilities = test::read_body_json(resp).await;
    assert_eq!(caps.default_cipher, "aegis256");
}

#[actix_web::test]
async fn test_admin_settings_rejects_unknown_cipher() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let req = test::TestRequest::get()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .to_request();
    let mut payload: Value = test::read_body_json(test::call_service(&app, req).await).await;

    for bad in ["aes256gcm", ""] {
        payload["sharing"]["default_cipher"] = Value::String(bad.to_string());
        let req = test::TestRequest::put()
            .uri("/api/admin/settings")
            .cookie(alice.jwt.clone())
            .set_json(&payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let caps: Capabilities = test::read_body_json(resp).await;
    assert_eq!(caps.default_cipher, "aegis128l");
}
