#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use hoodik::server;
use settings::{data::Users, factory::Factory};

#[actix_web::test]
async fn test_allow_register_false() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": false,
        "enforce_email_activation": false
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[actix_web::test]
async fn test_whitelist_pass_and_fail() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": false,
        "enforce_email_activation": false,
        "email_whitelist": {
            "rules": [
                "*@doe.com"
            ]
        }
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn test_registration_allowed_fails_blacklist_cannot_register() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": true,
        "enforce_email_activation": false,
        "email_blacklist": {
            "rules": [
                "*@doe.com"
            ]
        }
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[actix_web::test]
async fn test_register_status_reflects_setting() {
    let context = context::Context::mock_sqlite().await;

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::get()
        .uri("/api/auth/register/status")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["allow_register"], serde_json::Value::Bool(true));

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": false,
        "enforce_email_activation": false
    }))
    .unwrap();
    context.settings.replace_inner(settings).await;

    let req = test::TestRequest::get()
        .uri("/api/auth/register/status")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["allow_register"], serde_json::Value::Bool(false));
}

#[actix_web::test]
async fn test_registration_not_allowed_fails_blacklist_cannot_register() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": true,
        "enforce_email_activation": false,
        "email_blacklist": {
            "rules": [
                "*@doe.com"
            ]
        },
        "email_whitelist": {
            "rules": [
                "terry@example.com"
            ]
        }
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "terry@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
