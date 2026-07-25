#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use entity::EntityTrait;
use hoodik::server;

async fn find_user(context: &context::Context, id: entity::Uuid) -> entity::users::Model {
    entity::users::Entity::find_by_id(id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
}

#[actix_web::test]
async fn test_register_stores_locale() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let mut body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    body["locale"] = serde_json::json!("hr");

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let user_id = entity::Uuid::parse_str(body["user"]["id"].as_str().unwrap()).unwrap();

    let user = find_user(&context, user_id).await;
    assert_eq!(user.locale.as_deref(), Some("hr"));
}

#[actix_web::test]
async fn test_register_rejects_unsupported_locale() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let mut body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    body["locale"] = serde_json::json!("xx");

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[actix_web::test]
async fn test_register_without_locale_leaves_it_unset() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let registered = helpers::register_curve25519(&app, "john@doe.com").await;

    let user = find_user(&context, registered.user_id).await;
    assert_eq!(user.locale, None);
}

#[actix_web::test]
async fn test_patch_me_updates_locale() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let registered = helpers::register_curve25519(&app, "john@doe.com").await;

    let req = test::TestRequest::patch()
        .uri("/api/users/me")
        .cookie(registered.jwt.clone())
        .set_json(serde_json::json!({ "locale": "de" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = find_user(&context, registered.user_id).await;
    assert_eq!(user.locale.as_deref(), Some("de"));
}

#[actix_web::test]
async fn test_patch_me_rejects_unsupported_locale() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let registered = helpers::register_curve25519(&app, "john@doe.com").await;

    let req = test::TestRequest::patch()
        .uri("/api/users/me")
        .cookie(registered.jwt.clone())
        .set_json(serde_json::json!({ "locale": "klingon" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let user = find_user(&context, registered.user_id).await;
    assert_eq!(user.locale, None);
}
