//! Save-to-my-drive (fork) tests.
//!
//! The fork endpoint registers the metadata for a re-keyed copy of a
//! shared file on the caller's drive. These tests exercise the
//! permission gate (Owner/Co-owner only), the audit-event signature,
//! and the post-revoke independence invariant: a fork survives the
//! revocation of the source share that enabled it.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use entity::{
    share_events, user_files, ColumnTrait, EntityTrait, QueryFilter, Uuid,
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
async fn test_co_owner_can_fork_shared_file_into_own_drive() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let source = create_file!(app, alice, "source-of-fork");
    grant!(app, alice, bob, ShareRoleEnum::CoOwner, source.id);

    let new_file_id = Uuid::new_v4();
    let body = fork_body(&bob, source.id, new_file_id, 1024, now_secs());
    let resp = post_fork!(app, bob, source.id, body);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let response: Value = test::read_body_json(resp).await;
    assert_eq!(response["file_id"], new_file_id.to_string());

    // The original file is unchanged: Alice still owns it, Bob still
    // has his Co-owner row on it.
    let alice_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(source.id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("Alice's owner row on source must survive fork");
    assert!(alice_row.is_owner);
    let bob_source_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(source.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("Bob's co-owner row on source must survive fork");
    assert_eq!(bob_source_row.share_role, "co-owner");

    // The new file is owned by Bob, lives at his root, has a fresh id.
    let bob_owner_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("Bob's owner row on the fork must exist");
    assert!(bob_owner_row.is_owner);
    assert_ne!(new_file_id, source.id);

    // The fork audit row attributes the action to the source file id —
    // not the fork copy — so the original owner sees who saved a copy.
    // Without this, the row would land only in Bob's
    // own audit log and Alice would have no provenance trail.
    let fork_audit = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("fork"))
        .filter(share_events::Column::FileId.eq(source.id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(fork_audit.len(), 1);
    assert_eq!(fork_audit[0].sender_id, Some(bob.user_id));
    assert!(fork_audit[0].sender_signature.is_some());

    // The row has to be visible to the source-file owner
    // through the public events endpoint, not just to the forker. The
    // `events_for_user` visibility filter unions "rows on files the
    // caller owns" — alice owns the source file so the fork row is in
    // her audit view.
    let req = test::TestRequest::get()
        .uri("/api/shares/events?limit=100&offset=0")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let events = body["events"].as_array().expect("events array");
    let fork_row = events
        .iter()
        .find(|row| row["action"] == "fork")
        .expect("alice should see the fork audit row on her source file");
    assert_eq!(fork_row["file_id"], source.id.to_string());
    assert_eq!(fork_row["sender_id"], bob.user_id.to_string());

    // The same row must also reach the forker themselves; otherwise
    // they'd lose their own provenance trail.
    let req = test::TestRequest::get()
        .uri("/api/shares/events?limit=100&offset=0")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let events = body["events"].as_array().expect("events array");
    assert!(
        events.iter().any(|row| row["action"] == "fork"),
        "bob should see his own fork audit row"
    );
}

#[actix_web::test]
async fn test_editor_cannot_fork_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let source = create_file!(app, alice, "fork-editor-403");
    grant!(app, alice, bob, ShareRoleEnum::Editor, source.id);

    let new_file_id = Uuid::new_v4();
    let body = fork_body(&bob, source.id, new_file_id, 1024, now_secs());
    let resp = post_fork!(app, bob, source.id, body);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_not_forkable");

    // No new file row should exist.
    let rows = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(rows.is_empty(), "no fork row should be inserted on 403");
}

#[actix_web::test]
async fn test_reader_cannot_fork_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let source = create_file!(app, alice, "fork-reader-403");
    grant!(app, alice, bob, ShareRoleEnum::Reader, source.id);

    let new_file_id = Uuid::new_v4();
    let body = fork_body(&bob, source.id, new_file_id, 1024, now_secs());
    let resp = post_fork!(app, bob, source.id, body);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let _ = context;
}

#[actix_web::test]
async fn test_fork_persists_after_source_revoke() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let source = create_file!(app, alice, "fork-survives-revoke");
    grant!(app, alice, bob, ShareRoleEnum::CoOwner, source.id);

    let new_file_id = Uuid::new_v4();
    let body = fork_body(&bob, source.id, new_file_id, 1024, now_secs());
    let resp = post_fork!(app, bob, source.id, body);
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Alice revokes Bob's Co-owner access to the source. The fork is
    // a separate file under Bob's ownership; it must persist.
    let revoke = build_revoke_body(&alice, &bob, source.id, ShareRoleEnum::CoOwner, now_secs());
    let resp = delete_share!(app, alice, source.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let bob_on_source = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(source.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(
        bob_on_source.is_none(),
        "Bob's source row should be revoked"
    );

    let bob_on_fork = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("Bob's fork row should survive revoke of source");
    assert!(bob_on_fork.is_owner);
}

