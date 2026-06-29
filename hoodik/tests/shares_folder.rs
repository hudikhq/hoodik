//! Folder-share fan-out: eager recursion, subtree validation,
//! per-recipient cascade.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use entity::{user_files, ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use serde_json::Value;

use crate::shares_common::*;
use serde_json::Value as JsonValue;

/// Stamp a `members_list_signature` over the standard "owner +
/// recipient" projection onto an existing envelope. Most tests in this
/// file share a single recipient as Reader/Editor/Co-owner of the
/// folder root — owner row is `co-owner` per the storage default.
fn inject_owner_plus_recipient_sig(
    mut envelope: JsonValue,
    folder_id: entity::Uuid,
    owner: &TestUser,
    recipient: &TestUser,
    recipient_role: ShareRoleEnum,
    signer: &TestUser,
    signed_at: i64,
) -> JsonValue {
    let members = vec![
        FolderListMemberSpec {
            user: owner,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: owner,
        },
        FolderListMemberSpec {
            user: recipient,
            share_role: recipient_role,
            is_owner: false,
            signed_by: owner,
        },
    ];
    let sig = sign_folder_member_list(folder_id, owner.user_id, &members, signer, signed_at);
    envelope
        .as_object_mut()
        .unwrap()
        .insert("members_list_signature".to_string(), sig);
    envelope
}

fn inject_owner_plus_recipient_sig_with_revoke(
    revoke: JsonValue,
    folder_id: entity::Uuid,
    owner: &TestUser,
    signer: &TestUser,
    signed_at: i64,
) -> JsonValue {
    let members = vec![FolderListMemberSpec {
        user: owner,
        share_role: ShareRoleEnum::CoOwner,
        is_owner: true,
        signed_by: owner,
    }];
    let sig = sign_folder_member_list(folder_id, owner.user_id, &members, signer, signed_at);
    let mut body = revoke;
    body.as_object_mut()
        .unwrap()
        .insert("members_list_signature".to_string(), sig);
    body
}

#[actix_web::test]
async fn test_folder_share_eager_recursion_creates_rows_for_all_descendants() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "folder-root");
    let child_one = create_child_file!(app, alice, "child-one", folder.id);
    let child_two = create_child_file!(app, alice, "child-two", folder.id);

    let entries = vec![
        (folder.id, b"wrap-folder".to_vec()),
        (child_one.id, b"wrap-c1".to_vec()),
        (child_two.id, b"wrap-c2".to_vec()),
    ];
    let members = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &bob, share_role: ShareRoleEnum::Editor, is_owner: false, signed_by: &alice },
    ];
    let envelope = build_folder_share_envelope_with_entries(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        entries,
        random_nonce(),
        now_secs(),
        &members,
        &alice,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bob_rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(
        bob_rows.len(),
        3,
        "Bob should have one row per file in the subtree"
    );
}

#[actix_web::test]
async fn test_folder_share_partial_subtree_rejected_with_entries_do_not_match_subtree() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "partial-root");
    let _child = create_child_file!(app, alice, "partial-child", folder.id);

    // Send only the root entry; server expects the full subtree.
    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        folder.id,
        vec![(folder.id, b"wrap".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "entries_do_not_match_subtree");
    let _ = context;
}

#[actix_web::test]
async fn test_folder_share_5000_files_succeeds_at_boundary() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "5000-root");
    let mut entries = Vec::with_capacity(5000);
    entries.push((folder.id, b"wrap-folder".to_vec()));
    for i in 0..4999 {
        let child = create_child_file!(app, alice, &format!("c-{i}"), folder.id);
        entries.push((child.id, b"wrap".to_vec()));
    }
    assert_eq!(entries.len(), 5000);

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        folder.id,
        entries,
        random_nonce(),
        now_secs(),
    );
    let envelope = inject_owner_plus_recipient_sig(
        envelope,
        folder.id,
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        &alice,
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let _ = context;
}

#[actix_web::test]
async fn test_folder_share_5001_files_rejected_with_entries_too_many() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "5001-root");
    let mut entries = Vec::with_capacity(5001);
    entries.push((folder.id, b"wrap-folder".to_vec()));
    for i in 0..5000 {
        let child = create_child_file!(app, alice, &format!("c-{i}"), folder.id);
        entries.push((child.id, b"wrap".to_vec()));
    }
    assert_eq!(entries.len(), 5001);

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        folder.id,
        entries,
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "entries_too_many");
    let _ = context;
}

#[actix_web::test]
async fn test_folder_share_role_inherited_on_each_descendant_row() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "inherit-root");
    let child = create_child_file!(app, alice, "inherit-child", folder.id);

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::CoOwner,
        folder.id,
        vec![
            (folder.id, b"wrap".to_vec()),
            (child.id, b"wrap2".to_vec()),
        ],
        random_nonce(),
        now_secs(),
    );
    let envelope = inject_owner_plus_recipient_sig(
        envelope,
        folder.id,
        &alice,
        &bob,
        ShareRoleEnum::CoOwner,
        &alice,
        now_secs(),
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let bob_rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(bob_rows.len(), 2);
    assert!(bob_rows.iter().all(|r| r.share_role == "co-owner"));
}

#[actix_web::test]
async fn test_folder_share_revoke_cascade_drops_descendants_for_recipient() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "revoke-root");
    let child = create_child_file!(app, alice, "revoke-child", folder.id);

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![
            (folder.id, b"wrap".to_vec()),
            (child.id, b"wrap2".to_vec()),
        ],
        random_nonce(),
        now_secs(),
    );
    let envelope = inject_owner_plus_recipient_sig(
        envelope,
        folder.id,
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        &alice,
        now_secs(),
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let revoke_ts = now_secs();
    let revoke = build_revoke_body(&alice, &bob, folder.id, ShareRoleEnum::Editor, revoke_ts);
    let revoke = inject_owner_plus_recipient_sig_with_revoke(
        revoke,
        folder.id,
        &alice,
        &alice,
        revoke_ts,
    );
    let resp = delete_share!(app, alice, folder.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let remaining = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        remaining.is_empty(),
        "every Bob row under the folder should be gone"
    );
}

#[actix_web::test]
async fn test_subfolder_in_shared_folder_inherits_share_membership() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "parent-root");
    let sub = create_folder!(app, alice, "sub-folder", folder.id);
    let leaf = create_child_file!(app, alice, "leaf-file", sub.id);

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![
            (folder.id, b"wrap".to_vec()),
            (sub.id, b"wrap2".to_vec()),
            (leaf.id, b"wrap3".to_vec()),
        ],
        random_nonce(),
        now_secs(),
    );
    let envelope = inject_owner_plus_recipient_sig(
        envelope,
        folder.id,
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        &alice,
        now_secs(),
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let bob_leaf = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(leaf.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(bob_leaf.is_some());
    assert_eq!(bob_leaf.unwrap().share_role, "editor");
}

#[actix_web::test]
async fn test_member_cannot_see_files_outside_shared_subtree() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let shared_folder = create_folder!(app, alice, "shared-folder");
    let shared_child = create_child_file!(app, alice, "shared-child", shared_folder.id);
    let private_file = create_file!(app, alice, "alice-private");

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        shared_folder.id,
        vec![
            (shared_folder.id, b"wrap".to_vec()),
            (shared_child.id, b"wrap2".to_vec()),
        ],
        random_nonce(),
        now_secs(),
    );
    let envelope = inject_owner_plus_recipient_sig(
        envelope,
        shared_folder.id,
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        &alice,
        now_secs(),
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    // Bob has no `user_files` row for Alice's private file.
    let bob_private_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(private_file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(bob_private_row.is_none());

    // Bob's metadata read on the private file is a 404
    // (no-access reads must not leak existence).
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", private_file.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let _ = context;
}
