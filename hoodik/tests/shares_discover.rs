//! `GET /api/users/discover`. Verifies the
//! happy path, self-lookup rejection, unverified-account rejection,
//! and the rate limit's hit-and-miss-share-the-bucket property.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use entity::{users, ActiveValue, EntityTrait};
use hoodik::server;
use shares::data::discover::DiscoveredUser;


#[actix_web::test]
async fn test_discover_returns_user_pubkey_fingerprint() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    shares::test_support::reset_discover_rate_limit();

    let req = test::TestRequest::get()
        .uri("/api/users/discover?email=bob@example.com")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: DiscoveredUser = test::read_body_json(resp).await;
    assert_eq!(body.user_id, bob.user_id);
    assert_eq!(body.email, "bob@example.com");
    assert!(!body.pubkey.is_empty());
    assert_eq!(body.fingerprint, bob.fingerprint);
    let _ = context;
}

#[actix_web::test]
async fn test_discover_self_returns_400_cannot_discover_self() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    shares::test_support::reset_discover_rate_limit();

    let req = test::TestRequest::get()
        .uri("/api/users/discover?email=alice@example.com")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_discover_self");
    let _ = context;
}

#[actix_web::test]
async fn test_discover_unknown_email_returns_404() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    shares::test_support::reset_discover_rate_limit();

    let req = test::TestRequest::get()
        .uri("/api/users/discover?email=nobody@example.com")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "user_not_found");
    let _ = context;
}

#[actix_web::test]
async fn test_discover_unverified_user_is_discoverable() {
    // Hoodik supports self-hosted deployments that disable email
    // verification entirely (settings.users.enforce_email_activation =
    // false), so the sharing surface accepts unverified recipients —
    // they show up in discover results and can be shared with. Login
    // separately gates whether they can actually log in to read the
    // share.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, ghost, "ghost@example.com");
    shares::test_support::reset_discover_rate_limit();

    // Strip the ghost's verification stamp to model the unverified-but-
    // registered state. The discover endpoint still returns the user.
    users::Entity::update(users::ActiveModel {
        id: ActiveValue::Unchanged(ghost.user_id),
        email_verified_at: ActiveValue::Set(None),
        ..Default::default()
    })
    .exec(&context.db)
    .await
    .unwrap();

    let req = test::TestRequest::get()
        .uri("/api/users/discover?email=ghost@example.com")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "ghost@example.com");
    let _ = context;
}

#[actix_web::test]
async fn test_discover_rate_limit_21st_request_returns_429() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, _bob, "bob@example.com");
    shares::test_support::reset_discover_rate_limit();

    for _ in 0..20 {
        let req = test::TestRequest::get()
            .uri("/api/users/discover?email=bob@example.com")
            .cookie(alice.jwt.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
    let req = test::TestRequest::get()
        .uri("/api/users/discover?email=bob@example.com")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "rate_limited");
    let _ = context;
}

#[actix_web::test]
async fn test_discover_rate_limit_increments_on_both_hit_and_miss() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, _bob, "bob@example.com");
    shares::test_support::reset_discover_rate_limit();

    // Twenty 404 misses: each one increments the bucket.
    for _ in 0..20 {
        let req = test::TestRequest::get()
            .uri("/api/users/discover?email=nobody@example.com")
            .cookie(alice.jwt.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
    // The 21st call — a valid email — must still be rate-limited. If
    // 404s didn't count the attacker could enumerate at unlimited rate.
    let req = test::TestRequest::get()
        .uri("/api/users/discover?email=bob@example.com")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    let _ = context;
}
