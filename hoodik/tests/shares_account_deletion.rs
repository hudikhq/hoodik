//! Account-deletion cascade. Verifies the pre-emit
//! audit-row ordering, the DB-level FK CASCADE on
//! `user_files.shared_by_user_id`, and the SET NULL on
//! `share_events.sender_id` / `recipient_id`.
//!
//! Tests drive the cascade by calling the shares pre-emit helper +
//! `users::Entity::delete_by_id` directly. The admin route runs the
//! same sequence (step ordering) plus the file-purge
//! pass; the file-purge is tested in `admin/src/tests/users.rs` and
//! out of scope here.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use entity::{
    share_events, user_files, users, ColumnTrait, EntityTrait, QueryFilter,
};
use hoodik::server;

use crate::shares_common::*;

async fn run_account_deletion(context: &context::Context, user_id: entity::Uuid) {
    let now = chrono::Utc::now().timestamp();
    shares::pre_emit_for_user_delete(&context.db, user_id, now)
        .await
        .expect("pre-emit audit rows");
    users::Entity::delete_by_id(user_id)
        .exec(&context.db)
        .await
        .expect("delete user row");
}

#[actix_web::test]
async fn test_account_deletion_drops_user_files_where_user_was_recipient() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let file = create_file!(app, alice, "deleted-recipient-target");
    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    run_account_deletion(&context, bob.user_id).await;

    // Bob's incoming-share row is gone (FK CASCADE on user_files.user_id).
    let remaining = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(remaining.is_empty());

    // Alice's owner row survives — bob's delete only touches bob's rows.
    let alice_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .one(&context.db)
        .await
        .unwrap();
    assert!(alice_row.is_some());

    let _ = context;
}

#[actix_web::test]
async fn test_account_deletion_drops_grants_user_made_via_db_cascade() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_user!(app, carol, "carol@example.com");
    let file = create_file!(app, alice, "co-owner-cascade-target");

    // Bob becomes Co-owner; Bob then re-shares to Carol.
    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);
    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrap-for-carol".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let carol_before = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(carol.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(carol_before.is_some());

    // Delete Bob — FK CASCADE on `user_files.shared_by_user_id` must
    // drop Carol's row even without an explicit application-level walk.
    run_account_deletion(&context, bob.user_id).await;

    let carol_after = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(carol.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(
        carol_after.is_none(),
        "carol's row should be dropped by FK cascade on shared_by_user_id"
    );
    let _ = context;
}

#[actix_web::test]
async fn test_account_deletion_emits_audit_rows_with_system_attribution() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_user!(app, carol, "carol@example.com");
    let file = create_file!(app, alice, "audit-emit-target");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);
    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrap-for-carol".to_vec())],
        random_nonce(),
        now_secs(),
    );
    assert_eq!(post_share!(app, bob, envelope).status(), StatusCode::CREATED);

    let before_count = share_events::Entity::find()
        .all(&context.db)
        .await
        .unwrap()
        .len();

    run_account_deletion(&context, bob.user_id).await;

    let after = share_events::Entity::find().all(&context.db).await.unwrap();
    assert!(after.len() > before_count, "pre-emit must have added rows");

    // The newly inserted cascade rows are system-attributed — the
    // pre-emit signs nothing and the FK SET NULL turns sender_id into
    // NULL once the user row is gone.
    let cascade_rows: Vec<_> = after
        .iter()
        .filter(|row| {
            row.action == "shared_by_co_owner_revoked"
                || (row.action == "revoke" && row.sender_id.is_none())
        })
        .collect();
    assert!(
        !cascade_rows.is_empty(),
        "expected at least one system-attributed cascade row"
    );
    for row in &cascade_rows {
        assert!(
            row.sender_signature.is_none(),
            "system-cascade rows carry NULL sender_signature"
        );
    }
    let _ = context;
}

#[actix_web::test]
async fn test_account_deletion_preserves_share_events_with_null_attribution() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let file = create_file!(app, alice, "audit-set-null-target");
    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let grant_row = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("grant"))
        .one(&context.db)
        .await
        .unwrap()
        .expect("grant audit row must exist");
    assert_eq!(grant_row.sender_id, Some(alice.user_id));
    assert_eq!(grant_row.recipient_id, Some(bob.user_id));

    // Deleting alice cascades her owned files; share_events with
    // file_id matching those files go via the cascade FK, but
    // share_events for files that survive (here we delete the share
    // owner, so the file goes too) stay. Use bob's account instead so
    // the file lives on after deletion.
    let _ = file;

    run_account_deletion(&context, bob.user_id).await;

    let surviving = share_events::Entity::find_by_id(grant_row.id)
        .one(&context.db)
        .await
        .unwrap()
        .expect("grant row should still exist after recipient deletion");
    assert_eq!(surviving.sender_id, Some(alice.user_id));
    assert!(
        surviving.recipient_id.is_none(),
        "recipient_id SET NULL when bob's row goes"
    );
    // Hash-chain bytes are unchanged.
    assert_eq!(surviving.this_event_hash, grant_row.this_event_hash);
    assert_eq!(surviving.prev_event_hash, grant_row.prev_event_hash);
    let _ = context;
}
