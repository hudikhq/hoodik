//! Replay coverage for signature login. A client that sends a random nonce and
//! timestamp signs `fingerprint:timestamp:nonce`; recording each accepted
//! nonce makes a captured body single-use while repeated logins with the same
//! key stay distinguishable from replays. Clients predating those fields sign
//! the deterministic minute bucket, which stays accepted with its known
//! limitation that a second same-bucket login is refused. Both formats and
//! both key-resolution paths (live fingerprint and key-transition fallback)
//! converge on the same used-nonce record.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::{
    data::signature::Signature,
    mock::generate_fingerprint_nonce,
};
use hoodik::server;

/// The wire format upgraded clients sign, pinned independently of the server's
/// own canonical builder.
fn client_nonce_body(fingerprint: &str, timestamp: i64, private_pem: &str) -> Signature {
    let nonce = entity::Uuid::new_v4().simple().to_string();
    let canonical = format!("{fingerprint}:{timestamp}:{nonce}");
    let signature = cryptfns::rsa::private::sign(&canonical, private_pem).unwrap();

    Signature {
        fingerprint: Some(fingerprint.to_string()),
        signature: Some(signature),
        timestamp: Some(timestamp),
        nonce: Some(nonce),
    }
}

async fn post_signature(
    app: &impl helpers::TestApp,
    body: &Signature,
    ip: &str,
) -> actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody> {
    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .insert_header(("cf-connecting-ip", ip))
        .set_json(body)
        .to_request();
    test::call_service(app, req).await
}

/// Both requests must land in the same minute bucket for the second one to be
/// a true replay; near the bucket edge, wait out the rollover first.
fn wait_out_bucket_edge() {
    let into_bucket = chrono::Utc::now().timestamp() % 60;
    if into_bucket >= 55 {
        std::thread::sleep(std::time::Duration::from_secs((61 - into_bucket) as u64));
    }
}

#[actix_web::test]
async fn a_captured_signature_login_cannot_be_replayed() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "replay-victim@example.com").await;

    wait_out_bucket_edge();
    let nonce = generate_fingerprint_nonce(&owner.rsa_fingerprint);
    let signature = cryptfns::rsa::private::sign(&nonce, &owner.rsa_private).unwrap();
    let body = Signature {
        fingerprint: Some(owner.rsa_fingerprint.clone()),
        signature: Some(signature),
        ..Default::default()
    };

    let resp = post_signature(&app, &body, "10.9.0.1").await;
    assert_eq!(resp.status(), StatusCode::OK);

    // The identical body again — what a middlebox capture or a mirrored
    // request would send within the same minute.
    let resp = post_signature(&app, &body, "10.9.0.1").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = test::read_body(resp).await;
    assert!(
        String::from_utf8_lossy(&body).contains("signature_replayed"),
        "the refusal must come from the replay guard, not signature verification"
    );
}

#[actix_web::test]
async fn distinct_signature_logins_are_not_refused_as_replays() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let alice = helpers::seed_legacy_user(&context.db, "replay-alice@example.com").await;
    let bob = helpers::seed_legacy_user(&context.db, "replay-bob@example.com").await;

    wait_out_bucket_edge();
    for user in [&alice, &bob] {
        let nonce = generate_fingerprint_nonce(&user.rsa_fingerprint);
        let signature = cryptfns::rsa::private::sign(&nonce, &user.rsa_private).unwrap();
        let body = Signature {
            fingerprint: Some(user.rsa_fingerprint.clone()),
            signature: Some(signature),
            ..Default::default()
        };
        let resp = post_signature(&app, &body, "10.9.1.1").await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

#[actix_web::test]
async fn repeated_logins_with_the_same_key_both_succeed() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "replay-repeat@example.com").await;

    // Same key, same second — the exact case the deterministic bucket nonce
    // could not tell apart from a replay. Fresh nonces make each attempt a
    // distinct signed payload, so both must pass.
    let timestamp = chrono::Utc::now().timestamp();
    for _ in 0..2 {
        let body = client_nonce_body(&owner.rsa_fingerprint, timestamp, &owner.rsa_private);
        let resp = post_signature(&app, &body, "10.9.2.1").await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}

#[actix_web::test]
async fn a_captured_client_nonce_login_cannot_be_replayed() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "replay-nonce-victim@example.com").await;

    let timestamp = chrono::Utc::now().timestamp();
    let body = client_nonce_body(&owner.rsa_fingerprint, timestamp, &owner.rsa_private);

    let resp = post_signature(&app, &body, "10.9.3.1").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = post_signature(&app, &body, "10.9.3.1").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = test::read_body(resp).await;
    assert!(
        String::from_utf8_lossy(&body).contains("signature_replayed"),
        "the refusal must come from the replay guard, not signature verification"
    );
}

#[actix_web::test]
async fn a_stale_client_nonce_timestamp_is_refused() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "replay-stale@example.com").await;

    // Correctly signed, but outside the accepted clock skew — a capture held
    // back and fired later must die on the timestamp check alone.
    let timestamp = chrono::Utc::now().timestamp() - 301;
    let body = client_nonce_body(&owner.rsa_fingerprint, timestamp, &owner.rsa_private);

    let resp = post_signature(&app, &body, "10.9.4.1").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body = test::read_body(resp).await;
    assert!(String::from_utf8_lossy(&body).contains("signature_expired"));
}
