//! Quota / stats tests.
//!
//! Quota is enforced against owner-attributed bytes only; shared-with-me
//! files don't count toward the recipient. `POST /api/storage/stats`
//! reports `used_space` — the owner-attributed total counted against quota.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use entity::{
    user_files, users, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Uuid,
};
use hoodik::server;
use serde_json::{json, Value};

use crate::shares_common::*;

fn fork_body(
    actor: &TestUser,
    source_file_id: Uuid,
    new_file_id: Uuid,
    size: i64,
    timestamp: i64,
) -> Value {
    let event_signature = sign_fork_event(actor, source_file_id, timestamp);
    json!({
        "new_file_id": new_file_id.to_string(),
        "encrypted_metadata": "encrypted-name-for-fork",
        "name_hash": format!("fork-hash-{}", new_file_id),
        "mime": "text/plain",
        "size": size,
        "chunks": 1,
        "sha256": "deadbeef",
        "cipher": "ascon128a",
        "encrypted_key": cryptfns::rsa::public::encrypt("deadbeef", &actor.public_pem).unwrap(),
        "event_signature": event_signature,
        "timestamp": timestamp,
    })
}

macro_rules! stats_response {
    ($app:expr, $user:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage/stats")
            .cookie($user.jwt.clone())
            .to_request();
        let resp = actix_web::test::call_service(&$app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::OK);
        let body: serde_json::Value = actix_web::test::read_body_json(resp).await;
        body
    }};
}

macro_rules! post_fork {
    ($app:expr, $caller:expr, $source_file_id:expr, $body:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri(&format!("/api/shares/{}/fork", $source_file_id))
            .cookie($caller.jwt.clone())
            .set_json(&$body)
            .to_request();
        actix_web::test::call_service(&$app, req).await
    }};
}

#[actix_web::test]
async fn test_stats_used_space_counts_owner_bytes_not_shared_in() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");

    // Alice owns a 1024-byte file and shares it with Bob.
    let file = create_file!(app, alice, "stats-roundtrip");
    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    // The owner is charged for the bytes; the recipient is not.
    let alice_stats = stats_response!(app, alice);
    assert_eq!(alice_stats["used_space"].as_i64().unwrap(), 1024);

    let bob_stats = stats_response!(app, bob);
    assert_eq!(bob_stats["used_space"].as_i64().unwrap(), 0);
}

#[actix_web::test]
async fn test_shared_with_me_does_not_count_toward_recipient_quota() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");

    // Cap Bob at exactly 1 KiB so any owner-attributed write past the
    // first byte would exceed the gate.
    set_user_quota(&context, bob.user_id, 1024).await;

    // Alice creates and shares two 1024-byte files. Both flow into Bob
    // as non-owner rows.
    let file_a = create_file!(app, alice, "quota-shared-a");
    let file_b = create_file!(app, alice, "quota-shared-b");
    grant!(app, alice, bob, ShareRoleEnum::Reader, file_a.id);
    grant!(app, alice, bob, ShareRoleEnum::Reader, file_b.id);

    let bob_stats = stats_response!(app, bob);
    assert_eq!(
        bob_stats["used_space"].as_i64().unwrap(),
        0,
        "shared-in bytes must not count toward the recipient's used_space"
    );

    // Bob's own 1 KiB file still fits the quota; the quota check is
    // gated against owner-attributed bytes (`used_space` = 0 here), not
    // the 2 KiB of shared-in content. The post asserts on the live DB
    // row's quota field via the authenticated extractor, so the JWT's
    // cached quota does not matter.
    let bob_quota_row = users::Entity::find_by_id(bob.user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        bob_quota_row.quota,
        Some(1024),
        "live user row carries the 1 KiB quota we set"
    );
}

#[actix_web::test]
async fn test_fork_quota_exceeded_returns_409() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let source = create_file!(app, alice, "fork-quota-target");
    grant!(app, alice, bob, ShareRoleEnum::CoOwner, source.id);

    // Bob has a strict 0-byte quota so any positive fork size hits the
    // gate immediately. Quota = 0 with `used_space = 0` and
    // `claimed_size = 1024` -> 0 + 1024 > 0 -> 409.
    set_user_quota(&context, bob.user_id, 0).await;

    let new_file_id = Uuid::new_v4();
    let resp = post_fork!(
        app,
        bob,
        source.id,
        fork_body(&bob, source.id, new_file_id, 1024, now_secs())
    );
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "fork_quota_exceeded");

    // The transaction must not have committed: no files / user_files
    // / share_events rows for the would-be fork.
    let rows = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(rows.is_empty(), "no fork row on quota-exceeded");
}

async fn set_user_quota(context: &context::Context, user_id: Uuid, quota_bytes: i64) {
    let active = users::ActiveModel {
        id: ActiveValue::Unchanged(user_id),
        quota: ActiveValue::Set(Some(quota_bytes)),
        ..Default::default()
    };
    users::Entity::update(active)
        .exec(&context.db)
        .await
        .expect("set user quota");
}
