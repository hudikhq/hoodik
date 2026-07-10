//! Login lockout coverage across the credential-guessing surfaces. Only failed
//! authentications are charged, so tests drive real wrong-password attempts to
//! trip the limiter and correct ones to prove they never do. Each test draws
//! its own email and source IP so the process-global windows never alias under
//! the parallel runner.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use helpers::TestApp;
use hoodik::server;
use serde_json::{json, Value};

/// Mirror `auth::rate_limit`: an identity is locked after this many failures in
/// the window, a source IP after the wider budget below.
const IDENTITY_LIMIT: usize = 10;
const IP_LIMIT: usize = 100;

async fn legacy_login(app: &impl TestApp, email: &str, password: &str, ip: &str) -> StatusCode {
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("cf-connecting-ip", ip))
        .set_json(json!({ "email": email, "password": password }))
        .to_request();
    test::call_service(app, req).await.status()
}

async fn opaque_start(app: &impl TestApp, email: &str, ip: &str) -> StatusCode {
    let start = cryptfns::opaque::client_login_start(b"guess-guess-guess").unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .insert_header(("cf-connecting-ip", ip))
        .set_json(json!({ "email": email, "credential_request": start.message }))
        .to_request();
    test::call_service(app, req).await.status()
}

async fn signature_login(app: &impl TestApp, fingerprint: &str, ip: &str) -> StatusCode {
    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .insert_header(("cf-connecting-ip", ip))
        .set_json(json!({ "fingerprint": fingerprint, "signature": "00" }))
        .to_request();
    test::call_service(app, req).await.status()
}

#[actix_web::test]
async fn legacy_login_locks_out_and_a_correct_password_is_still_refused() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let email = "rl-legacy-lock@example.com";
    helpers::seed_legacy_user(&context.db, email).await;
    let ip = "10.1.0.1";

    for _ in 0..IDENTITY_LIMIT {
        assert_ne!(
            legacy_login(&app, email, "wrong-password", ip).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    // The window is full of failures: even the real password now bounces off the
    // limiter, so the lockout is a real gate and not just a failure counter.
    assert_eq!(
        legacy_login(&app, email, helpers::LEGACY_PASSWORD, ip).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    let _ = context;
}

#[actix_web::test]
async fn legitimate_correct_logins_never_trip() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let email = "rl-legacy-success@example.com";
    helpers::seed_legacy_user(&context.db, email).await;
    let ip = "10.2.0.1";

    // Well past the identity budget, all with the right password. Successes are
    // never charged, or a user re-logging in from several devices could lock
    // themselves out and the honest path would leak account activity.
    for _ in 0..(IDENTITY_LIMIT + 5) {
        assert_eq!(
            legacy_login(&app, email, helpers::LEGACY_PASSWORD, ip).await,
            StatusCode::OK
        );
    }
    let _ = context;
}

#[actix_web::test]
async fn opaque_login_start_is_never_throttled() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let email = "rl-opaque@example.com";
    let ip = "10.3.0.1";

    // start carries no secret and always succeeds, so it is deliberately
    // uncharged — the OPAQUE throttle lives at the failing login/finish.
    for _ in 0..(IDENTITY_LIMIT + 5) {
        assert_ne!(
            opaque_start(&app, email, ip).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    let _ = context;
}

#[actix_web::test]
async fn signature_login_locks_out() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let fingerprint = "a".repeat(64);
    let ip = "10.4.0.1";

    for _ in 0..IDENTITY_LIMIT {
        assert_ne!(
            signature_login(&app, &fingerprint, ip).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    assert_eq!(
        signature_login(&app, &fingerprint, ip).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    let _ = context;
}

#[actix_web::test]
async fn the_identity_window_trips_across_many_source_ips() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let email = "rl-victim@example.com";

    // Every failure comes from a different address, so no single IP window fills
    // — only the shared identity window does. This is the botnet case per-IP-only
    // limiting would miss.
    for i in 0..IDENTITY_LIMIT {
        let ip = format!("10.5.{i}.1");
        assert_ne!(
            legacy_login(&app, email, "wrong-password", &ip).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
    assert_eq!(
        legacy_login(&app, email, "wrong-password", "10.5.200.1").await,
        StatusCode::TOO_MANY_REQUESTS
    );
    let _ = context;
}

#[actix_web::test]
async fn a_full_source_ip_window_refuses_the_next_login() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    use auth::test_support::auth_rate_limit_charge_failure as charge_failure;
    let ip = "10.60.0.1";
    let now = chrono::Utc::now().timestamp();

    // Fill the source-IP window with failures that name no identity, standing in
    // for a single host spraying one password across the whole user table (which
    // as real HTTP calls would be IP_LIMIT slow bcrypt verifications).
    for _ in 0..IP_LIMIT {
        charge_failure(None, ip, now);
    }

    // A brand-new account from that address is refused on the IP window alone.
    assert_eq!(
        legacy_login(&app, "rl-spray-fresh@example.com", "wrong-password", ip).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    let _ = context;
}

#[actix_web::test]
async fn the_window_slides_forward() {
    use auth::test_support::{
        auth_rate_limit_charge_failure as charge_failure, auth_rate_limit_check as check,
    };

    let id = Some("slide-identity");
    let ip = "slide-source";

    for _ in 0..IDENTITY_LIMIT {
        charge_failure(id, ip, 0);
    }
    assert!(check(id, ip, 0).is_err());
    // A full window later, every earlier failure has aged out.
    assert!(check(id, ip, 301).is_ok());
}

async fn drive_past_limit(app: &impl TestApp, email: &str, ip: &str) -> (StatusCode, Value) {
    for _ in 0..IDENTITY_LIMIT {
        let _ = legacy_login(app, email, "wrong-password", ip).await;
    }
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("cf-connecting-ip", ip))
        .set_json(json!({ "email": email, "password": "wrong-password" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    (status, body)
}

#[actix_web::test]
async fn unknown_and_known_emails_are_indistinguishable_at_the_limit() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let known = "rl-known@example.com";
    helpers::seed_legacy_user(&context.db, known).await;

    let (known_status, known_body) = drive_past_limit(&app, known, "10.7.0.1").await;
    let (unknown_status, unknown_body) =
        drive_past_limit(&app, "rl-nobody@example.com", "10.7.0.2").await;

    assert_eq!(known_status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(unknown_status, StatusCode::TOO_MANY_REQUESTS);
    // Same status and same body — wrong passwords throttle a real account and an
    // unknown one identically, so the limiter leaks nothing about which exists.
    assert_eq!(known_body, unknown_body);
    let _ = context;
}
