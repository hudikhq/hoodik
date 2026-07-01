//! Recipient-side traversal of shared folders via the storage routes.
//!
//! A recipient who has a folder shared with them must be able to GET
//! that folder's metadata, list its children, and walk its breadcrumb
//! trail. The `find` and `metadata` routes were previously owner-only;
//! these
//! tests pin the new behavior so a future tightening can't silently
//! break the Shared-with-me click-through.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use hoodik::server;
use serde_json::Value;
use shares::data::incoming::IncomingSharePage;
use storage::data::{app_file::AppFile, response::Response as StorageResponse};

use crate::shares_common::*;

/// Reuses the `inject_owner_plus_recipient_sig` helper inline so this
/// test file stays decoupled from `shares_folder.rs`'s private helpers.
fn signed_folder_envelope(
    owner: &TestUser,
    recipient: &TestUser,
    role: ShareRoleEnum,
    folder_id: entity::Uuid,
    entries: Vec<(entity::Uuid, Vec<u8>)>,
) -> serde_json::Value {
    let now = now_secs();
    let envelope = build_share_envelope(
        owner,
        recipient,
        role,
        folder_id,
        entries,
        random_nonce(),
        now,
    );
    let members = vec![
        FolderListMemberSpec {
            user: owner,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: owner,
        },
        FolderListMemberSpec {
            user: recipient,
            share_role: role,
            is_owner: false,
            signed_by: owner,
        },
    ];
    let sig = sign_folder_member_list(folder_id, owner.user_id, &members, owner, now);
    let mut envelope = envelope;
    envelope
        .as_object_mut()
        .unwrap()
        .insert("members_list_signature".to_string(), sig);
    envelope
}

#[actix_web::test]
async fn test_recipient_can_fetch_metadata_for_shared_folder() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "shared-folder");
    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![(folder.id, b"wrap".to_vec())],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "non-owner recipient must be able to fetch metadata for a shared folder"
    );
    let body: AppFile = test::read_body_json(resp).await;
    assert_eq!(body.id, folder.id);
    assert!(!body.is_owner, "metadata reports the row as a non-owner row");
    assert_eq!(body.mime, "dir");
    let _ = context;
}

#[actix_web::test]
async fn test_recipient_lists_shared_folder_contents_without_is_owner_param() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "browse-target");
    let child_one = create_child_file!(app, alice, "doc-one", folder.id);
    let child_two = create_child_file!(app, alice, "doc-two", folder.id);

    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![
            (folder.id, b"wrap-folder".to_vec()),
            (child_one.id, b"wrap-one".to_vec()),
            (child_two.id, b"wrap-two".to_vec()),
        ],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    // Bob lists the folder the same way the web frontend does — no
    // `is_owner` param, just `dir_id` of the shared folder.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;
    let ids: std::collections::HashSet<_> = body.children.iter().map(|c| c.id).collect();
    assert!(
        ids.contains(&child_one.id),
        "shared file one must be listed"
    );
    assert!(
        ids.contains(&child_two.id),
        "shared file two must be listed"
    );
    assert!(
        body.children.iter().all(|c| !c.is_owner),
        "Bob is a non-owner of every shared row"
    );

    // The breadcrumb trail (`parents`) stops at the shared root because
    // Bob has no row for any ancestor above it.
    assert_eq!(body.parents.len(), 1);
    assert_eq!(body.parents[0].id, folder.id);
    assert!(!body.parents[0].is_owner);
    let _ = context;
}

#[actix_web::test]
async fn test_recipient_subfolder_breadcrumbs_stop_at_shared_root() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    // Alice's tree: outer-private / shared-root / sub / leaf.txt
    // Alice shares `shared-root` (and its descendants) with Bob. Bob's
    // breadcrumbs from `sub` must walk back only to `shared-root` —
    // never up into `outer-private`, which Bob has no row for.
    let outer = create_folder!(app, alice, "outer-private");
    let root = create_folder!(app, alice, "shared-root", outer.id);
    let sub = create_folder!(app, alice, "sub", root.id);
    let leaf = create_child_file!(app, alice, "leaf.txt", sub.id);

    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        root.id,
        vec![
            (root.id, b"wrap-root".to_vec()),
            (sub.id, b"wrap-sub".to_vec()),
            (leaf.id, b"wrap-leaf".to_vec()),
        ],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", sub.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;

    let parent_ids: Vec<_> = body.parents.iter().map(|p| p.id).collect();
    assert_eq!(
        parent_ids,
        vec![root.id, sub.id],
        "breadcrumb trail must stop at the shared root, never expose `outer`",
    );
    assert!(
        body.parents.iter().all(|p| !p.is_owner),
        "Bob's view of the trail is non-owner end-to-end"
    );
    assert!(
        !body.children.is_empty(),
        "sub-folder still lists its leaf for Bob"
    );
    assert_eq!(body.children[0].id, leaf.id);
    let _ = context;
}

#[actix_web::test]
async fn test_root_listing_unchanged_when_is_owner_not_specified() {
    // The recipient-traversal change only loosens the filter when
    // `dir_id` is set. The root view (`dir_id` unset) keeps the
    // owner-only default so Shared-with-me remains the canonical
    // surface for shared content.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let owned = create_file!(app, bob, "bob-owned");
    let alice_file = create_file!(app, alice, "alice-shared-to-bob");
    grant!(app, alice, bob, ShareRoleEnum::Reader, alice_file.id);

    let req = test::TestRequest::get()
        .uri("/api/storage")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;
    let ids: std::collections::HashSet<_> = body.children.iter().map(|c| c.id).collect();
    assert!(
        ids.contains(&owned.id),
        "Bob's owned root file must still appear"
    );
    assert!(
        !ids.contains(&alice_file.id),
        "Alice's shared file must not pollute Bob's root view"
    );
    let _ = context;
}

#[actix_web::test]
async fn test_incoming_share_carries_mime_for_folder_and_file() {
    // The `mime` field on `IncomingShare` is what lets the recipient
    // UI route a row click to either the file browser (`"dir"`) or
    // the file viewer (anything else). Pin both branches so a server-
    // side change to the join layout can't silently strip the mime.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let leaf = create_file!(app, alice, "shared-pdf");
    grant!(app, alice, bob, ShareRoleEnum::Reader, leaf.id);

    let folder = create_folder!(app, alice, "shared-dir");
    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![(folder.id, b"wrap".to_vec())],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let page: IncomingSharePage = test::read_body_json(resp).await;

    let leaf_row = page
        .items
        .iter()
        .find(|i| i.file_id == leaf.id)
        .expect("incoming list must contain the shared file");
    assert_eq!(
        leaf_row.mime, "text/plain",
        "leaf-row mime mirrors the underlying file row, not the folder default"
    );

    let folder_row = page
        .items
        .iter()
        .find(|i| i.file_id == folder.id)
        .expect("incoming list must contain the shared folder");
    assert_eq!(folder_row.mime, "dir");

    // Non-folder rows must not advertise as dirs — the UI relies on
    // this to drive the click-handler branch.
    assert_ne!(leaf_row.mime, "dir");
    let _ = context;
}

#[actix_web::test]
async fn test_explicit_is_owner_false_query_still_works() {
    // The Audit and admin views pass `is_owner=false` explicitly to
    // ask for shared-only content. The new logic must keep honouring
    // that — only the no-param-with-dir-id branch is new behaviour.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "explicit-folder");
    let child = create_child_file!(app, alice, "explicit-child", folder.id);
    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![
            (folder.id, b"wrap".to_vec()),
            (child.id, b"wrap-c".to_vec()),
        ],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri(&format!(
            "/api/storage?dir_id={}&is_owner=false",
            folder.id
        ))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;
    assert!(
        body.children.iter().any(|c| c.id == child.id),
        "explicit is_owner=false should keep returning the recipient's shared children"
    );
    let _ = context;
}

/// Sanity: when Alice (owner) lists her own folder, none of the new
/// behavior changes her view — she still sees only her owned rows and
/// the breadcrumb trail walks her ancestors.
#[actix_web::test]
async fn test_owner_listing_unaffected_by_recipient_changes() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");

    let folder = create_folder!(app, alice, "alice-folder");
    let child = create_child_file!(app, alice, "alice-child", folder.id);

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;
    assert_eq!(body.children.len(), 1);
    assert_eq!(body.children[0].id, child.id);
    assert!(body.children[0].is_owner);
    assert_eq!(body.parents.len(), 1);
    assert_eq!(body.parents[0].id, folder.id);
    assert!(body.parents[0].is_owner);

    // And metadata for her own folder still works — same behaviour
    // as before the route was opened to non-owners.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["mime"], "dir");
    assert_eq!(body["is_owner"], true);
    let _ = context;
}

/// Inside a shared folder, every non-owner row carries the email of the
/// user who actually owns the file row. Owned rows skip the lookup so the
/// recipient doesn't see their own address on their own files.
#[actix_web::test]
async fn test_shared_folder_listing_carries_owner_email_per_row() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "shared-folder-owners");
    let alice_child = create_child_file!(app, alice, "alice-child", folder.id);
    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![
            (folder.id, b"wrap-folder".to_vec()),
            (alice_child.id, b"wrap-alice".to_vec()),
        ],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;

    let row = body
        .children
        .iter()
        .find(|c| c.id == alice_child.id)
        .expect("Alice's file must surface in Bob's listing of the shared folder");
    assert!(!row.is_owner);
    assert_eq!(
        row.owner_email.as_deref(),
        Some("alice@example.com"),
        "non-owner row must surface the file owner's email"
    );

    // The folder itself (Alice's row, Bob's view) also carries her email.
    assert_eq!(
        body.parents[0].owner_email.as_deref(),
        Some("alice@example.com")
    );

    // Alice's own listing of the folder — she's the owner of every row,
    // so `owner_email` stays None (avoids leaking her own address back).
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: StorageResponse = test::read_body_json(resp).await;
    let row = body
        .children
        .iter()
        .find(|c| c.id == alice_child.id)
        .expect("Alice still sees her own file");
    assert!(row.is_owner);
    assert!(
        row.owner_email.is_none(),
        "owner doesn't get their own email echoed back"
    );
    let _ = context;
}

/// `GET /api/storage/{id}/metadata` carries the same `owner_email` field
/// so a recipient navigating directly into a deep-link knows whose folder
/// they're sitting in.
#[actix_web::test]
async fn test_metadata_endpoint_carries_owner_email_for_non_owner_caller() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "metadata-owner-email");
    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        folder.id,
        vec![(folder.id, b"wrap".to_vec())],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: AppFile = test::read_body_json(resp).await;
    assert!(!body.is_owner);
    assert_eq!(body.owner_email.as_deref(), Some("alice@example.com"));

    // Alice asks for her own folder's metadata — no echo of her own email.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: AppFile = test::read_body_json(resp).await;
    assert!(body.is_owner);
    assert!(body.owner_email.is_none());
    let _ = context;
}

/// `shared_with_count` mirrors the number of non-owner `user_files` rows
/// the file row carries. Owner-side listings surface it so the SPA can
/// render a "shared with N others" hint inline; recipient-side rows
/// stay at zero because the badge is owner-only information.
#[actix_web::test]
async fn test_storage_listing_carries_shared_with_count_for_owner_rows() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "shared-out");
    let alice_child = create_child_file!(app, alice, "alice-child", folder.id);

    // Alice has not shared the file yet — count is zero in her own listing.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: StorageResponse = test::read_body_json(resp).await;
    let row = body
        .children
        .iter()
        .find(|c| c.id == alice_child.id)
        .expect("Alice sees her child");
    assert_eq!(row.shared_with_count, 0);

    // Share with Bob and re-check the count.
    let envelope = signed_folder_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![
            (folder.id, b"wrap-folder-bob".to_vec()),
            (alice_child.id, b"wrap-child-bob".to_vec()),
        ],
    );
    assert!(post_share!(app, alice, envelope).status().is_success());

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: StorageResponse = test::read_body_json(resp).await;
    let row = body
        .children
        .iter()
        .find(|c| c.id == alice_child.id)
        .expect("Alice still sees her child");
    assert_eq!(row.shared_with_count, 1);
    let parent = body
        .parents
        .iter()
        .find(|p| p.id == folder.id)
        .expect("folder appears as a parent");
    assert_eq!(parent.shared_with_count, 1);

    // Bob's listing of the same folder: he isn't the owner, so the
    // count stays zero on every row he sees.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage?dir_id={}", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: StorageResponse = test::read_body_json(resp).await;
    let row = body
        .children
        .iter()
        .find(|c| c.id == alice_child.id)
        .expect("Bob sees Alice's child via the share");
    assert!(!row.is_owner);
    assert_eq!(row.shared_with_count, 0);

    // Metadata endpoint covers the same plumbing for a direct deep-link.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", alice_child.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: AppFile = test::read_body_json(resp).await;
    assert_eq!(body.shared_with_count, 1);

    let _ = context;
}
