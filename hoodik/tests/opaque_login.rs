//! End-to-end OPAQUE endpoint tests. `cryptfns::opaque` plays the client, so
//! the password only ever exists locally: the server sees registration and
//! login messages but never the password itself.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test};
use entity::{opaque_ksf, EntityTrait};
use hoodik::server;
use serde_json::{json, Value};

const EMAIL: &str = "opaque@example.com";
const PASSWORD: &[u8] = helpers::LEGACY_PASSWORD.as_bytes();

/// The service produced by `test::init_service(server::app(..))`.
trait TestApp:
    Service<actix_http::Request, Response = ServiceResponse<Self::Body>, Error = actix_web::Error>
{
    type Body: actix_web::body::MessageBody;
}

impl<S, B> TestApp for S
where
    B: actix_web::body::MessageBody,
    S: Service<
        actix_http::Request,
        Response = ServiceResponse<B>,
        Error = actix_web::Error,
    >,
{
    type Body = B;
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

/// Drive the two-step OPAQUE login. A wrong password is caught client-side at
/// `client_login_finish` (the client can't complete the KE), so that returns
/// `None`; otherwise it returns the server's login-finish response.
async fn opaque_login(
    app: &impl TestApp,
    password: &[u8],
) -> Option<ServiceResponse<impl actix_web::body::MessageBody>> {
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

    // A v2 account registers OPAQUE at signup; the password never crosses the
    // wire, yet the OPAQUE login below completes.
    helpers::register_curve25519(&app, EMAIL).await;

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

    helpers::register_curve25519(&app, EMAIL).await;

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

/// `login/start` for a migrated account returns its stored KSF parameters so
/// the client stretches the password with the values registration used.
#[actix_web::test]
async fn test_login_start_returns_ksf_params() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    helpers::register_curve25519(&app, EMAIL).await;

    let start = cryptfns::opaque::client_login_start(PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .insert_header(("cf-connecting-ip", "ksf-params-test"))
        .set_json(json!({ "email": EMAIL, "credential_request": start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body["method"], "opaque");
    let ksf = &body["ksf"];
    assert_eq!(ksf["algorithm"], "argon2id");
    assert_eq!(ksf["m_cost"], 65536);
    assert_eq!(ksf["t_cost"], 3);
    assert_eq!(ksf["p_cost"], 1);
    assert_eq!(ksf["protocol_version"], 1);
}

/// An unknown email and a legacy account both answer `password` with the same
/// default KSF — the parameters add no new enumeration signal.
#[actix_web::test]
async fn test_unknown_and_legacy_ksf_are_indistinguishable() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    seed_and_login_legacy(&app, &context.db).await;

    let ksf_for = |email: &str| {
        let start = cryptfns::opaque::client_login_start(PASSWORD).unwrap();
        test::TestRequest::post()
            .uri("/api/auth/login/start")
            .insert_header(("cf-connecting-ip", email))
            .set_json(json!({ "email": email, "credential_request": start.message }))
            .to_request()
    };

    let legacy: Value = test::read_body_json(test::call_service(&app, ksf_for(EMAIL)).await).await;
    let unknown: Value =
        test::read_body_json(test::call_service(&app, ksf_for("nobody@example.com")).await).await;

    assert_eq!(legacy["method"], "password");
    assert_eq!(unknown["method"], "password");
    assert_eq!(legacy["ksf"], unknown["ksf"], "same KSF for legacy and unknown");
    assert_eq!(legacy["ksf"]["m_cost"], 65536);
}

/// The client can drive the finish with the parameters `login/start` returned
/// and still log in — the per-user-parameter path works end to end.
#[actix_web::test]
async fn test_login_finish_with_returned_params_succeeds() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    helpers::register_curve25519(&app, EMAIL).await;

    let start = cryptfns::opaque::client_login_start(PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .insert_header(("cf-connecting-ip", "finish-params-test"))
        .set_json(json!({ "email": EMAIL, "credential_request": start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    let login_id = body["login_id"].as_str().unwrap();
    let credential_response = body["credential_response"].as_str().unwrap();
    let ksf = &body["ksf"];

    let finish = cryptfns::opaque::client_login_finish_with_params(
        &start.state,
        credential_response,
        PASSWORD,
        ksf["m_cost"].as_u64().unwrap() as u32,
        ksf["t_cost"].as_u64().unwrap() as u32,
        ksf["p_cost"].as_u64().unwrap() as u32,
    )
    .expect("finish with returned params");

    let req = test::TestRequest::post()
        .uri("/api/auth/login/finish")
        .set_json(json!({ "login_id": login_id, "credential_finalization": finish.finalization }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "login with returned KSF params succeeds");
}

/// Every account row carries the current KSF constants — the column default
/// that also backfills accounts that predate the parameter columns.
#[actix_web::test]
async fn test_account_row_holds_current_ksf_constants() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let registered = helpers::register_curve25519(&app, EMAIL).await;

    let row = opaque_ksf::Entity::find_by_id(registered.user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.ksf_algorithm, "argon2id");
    assert_eq!(row.ksf_m_cost, 65536);
    assert_eq!(row.ksf_t_cost, 3);
    assert_eq!(row.ksf_p_cost, 1);
    assert_eq!(row.opaque_protocol_version, 1);
}
