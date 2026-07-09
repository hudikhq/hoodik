//! End-to-end OPAQUE endpoint tests. `cryptfns::opaque` plays the client, so
//! the password only ever exists locally: the server sees registration and
//! login messages but never the password itself.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test};
use hoodik::server;
use serde_json::{json, Value};

const EMAIL: &str = "opaque@example.com";
const PASSWORD: &[u8] = helpers::LEGACY_PASSWORD.as_bytes();

/// The service produced by `test::init_service(server::app(..))`.
trait TestApp:
    Service<actix_http::Request, Response = ServiceResponse<EitherBody<BoxBody>>, Error = actix_web::Error>
{
}

impl<S> TestApp for S where
    S: Service<
        actix_http::Request,
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = actix_web::Error,
    >
{
}

/// Seed a legacy RSA account at the data layer and log it in for a session
/// cookie. Legacy accounts are no longer created through registration, but the
/// OPAQUE endpoints must keep serving the pre-migration → migrated transition.
async fn seed_and_login_legacy(
    app: &impl TestApp,
    db: &entity::DbConn,
) -> actix_web::cookie::Cookie<'static> {
    helpers::seed_legacy_user(db, EMAIL).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": EMAIL, "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    jwt.unwrap()
}

/// Drive the two-step OPAQUE registration through the authenticated endpoints.
async fn opaque_register(app: &impl TestApp, jwt: &actix_web::cookie::Cookie<'static>) {
    let start = cryptfns::opaque::client_registration_start(PASSWORD).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/start")
        .cookie(jwt.clone())
        .set_json(json!({ "registration_request": start.message }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let registration_response = body["registration_response"].as_str().unwrap();

    let finish =
        cryptfns::opaque::client_registration_finish(&start.state, registration_response, PASSWORD)
            .unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/finish")
        .cookie(jwt.clone())
        .set_json(json!({
            "registration_upload": finish.message,
            "encrypted_private_key": "envelope-stand-in",
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

/// Drive the two-step OPAQUE login. A wrong password is caught client-side at
/// `client_login_finish` (the client can't complete the KE), so that returns
/// `None`; otherwise it returns the server's login-finish response.
async fn opaque_login(
    app: &impl TestApp,
    password: &[u8],
) -> Option<ServiceResponse<EitherBody<BoxBody>>> {
    let start = cryptfns::opaque::client_login_start(password).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .set_json(json!({ "email": EMAIL, "credential_request": start.message }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["method"], "opaque", "migrated account must offer OPAQUE");

    let login_id = body["login_id"].as_str().unwrap();
    let credential_response = body["credential_response"].as_str().unwrap();

    let finish =
        cryptfns::opaque::client_login_finish(&start.state, credential_response, password).ok()?;

    let req = test::TestRequest::post()
        .uri("/api/auth/login/finish")
        .set_json(json!({
            "login_id": login_id,
            "credential_finalization": finish.finalization,
        }))
        .to_request();
    Some(test::call_service(app, req).await)
}

#[actix_web::test]
async fn test_opaque_register_then_login() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = seed_and_login_legacy(&app, &context.db).await;
    opaque_register(&app, &jwt).await;

    let resp = opaque_login(&app, PASSWORD)
        .await
        .expect("correct password completes the client KE");
    assert_eq!(resp.status(), StatusCode::OK, "correct password logs in via OPAQUE");
    let (jwt, refresh) = helpers::extract_cookies(resp.headers());
    assert!(jwt.is_some() && refresh.is_some(), "OPAQUE login issues session cookies");
}

#[actix_web::test]
async fn test_wrong_password_rejected() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = seed_and_login_legacy(&app, &context.db).await;
    opaque_register(&app, &jwt).await;

    // A wrong password fails: either client-side (the KE can't complete, None)
    // or, if a finalization is somehow produced, the server rejects it.
    match opaque_login(&app, b"a-different-password").await {
        None => {}
        Some(resp) => assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "a wrong password must never yield a session"
        ),
    }
}

#[actix_web::test]
async fn test_legacy_account_offers_password_method() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // Registered but never OPAQUE-migrated.
    seed_and_login_legacy(&app, &context.db).await;

    let start = cryptfns::opaque::client_login_start(PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .set_json(json!({ "email": EMAIL, "credential_request": start.message }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["method"], "password", "legacy account routes to password login");
}

#[actix_web::test]
async fn test_unknown_email_does_not_leak_existence() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let start = cryptfns::opaque::client_login_start(PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .set_json(json!({ "email": "nobody@example.com", "credential_request": start.message }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["method"], "password", "unknown email is indistinguishable from legacy");
}
