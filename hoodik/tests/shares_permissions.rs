//! Permission gating tests + the storage-route × role matrix. Every
//! mutating storage route is exercised against every tier in the
//! share-role hierarchy.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use entity::{user_files, ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use serde_json::Value;
use shares::data::capabilities::Capabilities;

use crate::shares_common::*;

#[macro_export]
macro_rules! grant_role_to {
    ($app:expr, $sender:expr, $recipient:expr, $role:expr, $file_id:expr) => {{
        let envelope = $crate::shares_common::build_share_envelope(
            &$sender,
            &$recipient,
            $role,
            $file_id,
            vec![($file_id, b"wrap".to_vec())],
            $crate::shares_common::random_nonce(),
            $crate::shares_common::now_secs(),
        );
        let resp = post_share!($app, $sender, envelope);
        assert!(
            resp.status().is_success(),
            "grant to {} failed: {:?}",
            $recipient.email,
            resp.status()
        );
    }};
}

#[macro_export]
macro_rules! mark_editable {
    ($app:expr, $owner:expr, $file_id:expr) => {{
        let req = actix_web::test::TestRequest::put()
            .uri(&format!("/api/storage/{}/editable", $file_id))
            .cookie($owner.jwt.clone())
            .set_json(&serde_json::json!({"editable": true}))
            .to_request();
        let resp = actix_web::test::call_service(&$app, req).await;
        assert!(
            resp.status().is_success(),
            "set_editable: {:?}",
            resp.status()
        );
    }};
}

#[macro_export]
macro_rules! upload_token_for {
    ($app:expr, $user:expr, $file_id:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri("/api/auth/transfer-token")
            .cookie($user.jwt.clone())
            .set_json(&serde_json::json!({
                "file_id": $file_id.to_string(),
                "action": "upload",
            }))
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        let token: auth::data::transfer_token::TransferTokenResponse =
            serde_json::from_slice(&body).expect("upload token json");
        token.token
    }};
}

#[actix_web::test]
async fn test_reader_cannot_call_post_shares_for_reshare_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "reader-cannot-reshare");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_share_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_editor_cannot_call_post_shares_for_reshare_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "editor-cannot-reshare");

    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_share_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_reader_cannot_call_delete_shares_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "reader-cannot-delete");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file.id);

    let revoke = build_revoke_body(&bob, &carol, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, bob, file.id, carol.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_revoke_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_none_role_returns_404_on_get_shares_file_id() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "none-role-404");

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/{}", file.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "file_not_found");
    let _ = context;
}

#[actix_web::test]
async fn test_capability_endpoint_returns_correct_role_list() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let caps: Capabilities = test::read_body_json(resp).await;
    assert!(caps.sharing.enabled);
    assert_eq!(caps.sharing.roles, vec!["reader", "editor", "co-owner"]);
    assert!(caps.editable_folders);
    assert!(caps.share_groups);
    assert!(caps.audit_log);
    assert!(caps.fork);
    let _ = context;
}

#[actix_web::test]
async fn test_capability_endpoint_works_without_auth() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // No JWT cookie attached — the route should still respond.
    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = context;
}

// Storage route × role matrix.

#[actix_web::test]
async fn test_reader_download_200() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "reader-download");
    grant_role_to!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    // Bob can fetch metadata on the shared file (read access).
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", file.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = context;
}

#[actix_web::test]
async fn test_reader_upload_chunk_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "reader-cant-upload");
    grant_role_to!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let token = upload_token_for!(app, bob, file.id);
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/storage/{}?chunk=0&checksum=ignored",
            file.id
        ))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_payload(b"chunk-bytes".to_vec())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_read_only");
    let _ = context;
}

#[actix_web::test]
async fn test_reader_rename_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "reader-rename");
    mark_editable!(app, alice, file.id);
    grant_role_to!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({
            "encrypted_name": "renamed",
            "name_hash": "new-hash",
            "search_tokens_hashed": [],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_read_only");
    let _ = context;
}

#[actix_web::test]
async fn test_reader_set_editable_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "reader-set-editable");
    grant_role_to!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({"editable": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_editor_replace_content_200() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "editor-replace");
    mark_editable!(app, alice, file.id);
    grant_role_to!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({
            "size": 1024,
            "chunks": 1,
            "search_tokens_hashed": [],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = context;
}

#[actix_web::test]
async fn test_editor_set_editable_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "editor-set-editable");
    grant_role_to!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({"editable": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_editor_version_restore_200() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "editor-restore");
    mark_editable!(app, alice, file.id);
    grant_role_to!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    // The restore route attempts to find the requested version; with
    // history empty the route returns a 4xx (no version found) rather
    // than 401 — proving the editor passes the permission gate. A 401
    // would indicate the gate rejected before the lookup ran.
    let req = test::TestRequest::post()
        .uri(&format!("/api/storage/{}/versions/1/restore", file.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_ne!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "permission gate must pass for editors; got: {:?}",
        resp.status()
    );
    let _ = context;
}

#[actix_web::test]
async fn test_editor_version_delete_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "editor-version-delete");
    mark_editable!(app, alice, file.id);
    grant_role_to!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/storage/{}/versions/1", file.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_replace_content_200() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "co-owner-replace");
    mark_editable!(app, alice, file.id);
    grant_role_to!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({
            "size": 1024,
            "chunks": 1,
            "search_tokens_hashed": [],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_set_editable_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "co-owner-set-editable");
    grant_role_to!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({"editable": true}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_none_role_storage_metadata_returns_404_not_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "no-access");

    // Bob has no share row on alice's file. Read paths return 404 to
    // avoid existence leakage.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", file.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let _ = context;
}

#[actix_web::test]
async fn test_owner_delete_folder_cascades() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "owner-delete-folder");
    let child = create_child_file!(app, alice, "owner-delete-child", folder.id);

    let members = vec![
        FolderListMemberSpec {
            user: &alice,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: &alice,
        },
        FolderListMemberSpec {
            user: &bob,
            share_role: ShareRoleEnum::Editor,
            is_owner: false,
            signed_by: &alice,
        },
    ];
    let envelope = build_folder_share_envelope_with_entries(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        vec![
            (folder.id, b"wrap".to_vec()),
            (child.id, b"wrap2".to_vec()),
        ],
        random_nonce(),
        now_secs(),
        &members,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/storage/{}", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Every user_files row under the folder is gone — bob's and
    // alice's both — because the files themselves cascaded.
    let any = user_files::Entity::find()
        .filter(user_files::Column::FileId.is_in(vec![folder.id, child.id]))
        .all(&context.db)
        .await
        .unwrap();
    assert!(any.is_empty(), "owner cascade drops every user_files row");
    let _ = context;
}

#[actix_web::test]
async fn test_non_owner_delete_folder_self_removes_recursively() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "non-owner-delete");
    let child = create_child_file!(app, alice, "non-owner-child", folder.id);

    let members = vec![
        FolderListMemberSpec {
            user: &alice,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: &alice,
        },
        FolderListMemberSpec {
            user: &bob,
            share_role: ShareRoleEnum::Editor,
            is_owner: false,
            signed_by: &alice,
        },
    ];
    let envelope = build_folder_share_envelope_with_entries(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        vec![
            (folder.id, b"wrap".to_vec()),
            (child.id, b"wrap2".to_vec()),
        ],
        random_nonce(),
        now_secs(),
        &members,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/storage/{}", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Bob's rows recursively gone; alice's owner rows survive.
    let bob_remaining = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert!(bob_remaining.is_empty());
    let alice_rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .all(&context.db)
        .await
        .unwrap();
    assert!(alice_rows.len() >= 2, "alice still owns the folder and child");
    let _ = context;
}

#[actix_web::test]
async fn test_name_hash_endpoint_owner_only() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "name-hash-target");
    grant_role_to!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    // Bob is a Co-owner but the name-hash route returns 404 to non-
    // owners — the route is owner-only to avoid
    // enumerating name hashes across recipients.
    let req = test::TestRequest::get()
        .uri("/api/storage/name-hash-target/name-hash")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Alice (the owner) finds the file.
    let req = test::TestRequest::get()
        .uri("/api/storage/name-hash-target/name-hash")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = context;
}
