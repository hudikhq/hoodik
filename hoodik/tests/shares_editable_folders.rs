//! Editable folder protocol. Multi-key
//! upload, delegated signing on Co-owner-added members, move-into-
//! shared re-wraps, and the cascade fan-out under Co-owner revoke.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::{AuditEventActionEnum, ShareRoleEnum};
use entity::{share_events, user_files, ColumnTrait, EntityTrait, QueryFilter, Uuid};
use hoodik::server;
use serde_json::Value;

use crate::shares_common::*;

/// Standard two-member roster (owner + recipient at the requested role).
/// Folder owner is also the signer because these helpers cover the
/// initial-share path; for Co-owner re-shares tests build the roster
/// inline so the signer pointer differs.
fn owner_plus_recipient<'a>(
    owner: &'a TestUser,
    recipient: &'a TestUser,
    recipient_role: ShareRoleEnum,
) -> Vec<FolderListMemberSpec<'a>> {
    vec![
        // Owner rows ship from storage with `share_role = "co-owner"`
        // (see storage::manage create_file). The server-side
        // canonicaliser reads that literal value back, so the test
        // helper has to project the same.
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
    ]
}

#[actix_web::test]
async fn test_editor_uploads_into_shared_folder_via_multikey_endpoint() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "editor-upload");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let member_keys = vec![
        (alice.user_id, "wrap-for-alice", false),
        (bob.user_id, "wrap-for-bob", true),
    ];
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "mk-upload",
        member_keys,
        timestamp,
        None,
        event_signature,
        timestamp,
    );

    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // The new file has owner=bob and recipient=alice (folder owner).
    let alice_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("alice row on the uploaded file");
    assert!(!alice_row.is_owner);
    let bob_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob row");
    assert!(bob_row.is_owner);
    let _ = context;
}

#[actix_web::test]
async fn test_owner_multikey_upload_fans_out_to_existing_members() {
    // Bug #2: when the owner uploads into a folder they've already
    // shared, the new file must be wrapped for every current member,
    // mirroring the recipient-driven upload-multikey path. The
    // frontend routes the owner through the same endpoint; the test
    // exercises the server contract that lets that happen.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "owner-fan-out");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Alice (the folder owner) uploads through the same multi-key
    // endpoint a recipient would use, claiming `is_owner_of_file=true`
    // for her own wrap.
    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "owner-mk",
        vec![
            (alice.user_id, "wrap-for-alice", true),
            (bob.user_id, "wrap-for-bob", false),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Both members have a row — alice owns the new file, bob inherits
    // an Editor row that lets him decrypt it with his RSA wrap.
    let bob_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob's row on owner-uploaded file");
    assert!(!bob_row.is_owner);
    assert_eq!(bob_row.share_role, "editor");
    let alice_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("alice's row");
    assert!(alice_row.is_owner);
}

#[actix_web::test]
async fn test_multikey_upload_then_chunk_upload_finalizes() {
    // Recipient's multi-key upload must follow the same finalize path
    // as a regular owner upload: chunk lands, `chunks_stored == chunks`
    // triggers `finish()`, and `finished_upload_at` is stamped. Without
    // this the UI shows "forever uploading" even though the bytes are
    // on disk.
    let context =
        context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "multikey-finalize");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "mk-finalize",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::CREATED);

    // Push the only chunk and assert the chunk endpoint stamps
    // `finished_upload_at` in the response — this is what tells the
    // upload queue the file is done.
    let chunk = b"hello-multikey".to_vec();
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?chunk=0", new_file_id).as_str())
        .cookie(bob.jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(chunk)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let response_body = test::read_body(resp).await;
    let parsed: serde_json::Value = serde_json::from_slice(&response_body).unwrap_or_else(|_| {
        panic!(
            "chunk upload response not JSON: status={}, body={}",
            status,
            String::from_utf8_lossy(&response_body)
        )
    });
    assert_eq!(status, StatusCode::OK, "chunk upload failed: {parsed}");
    assert_eq!(parsed["chunks_stored"], 1);
    assert!(
        !parsed["finished_upload_at"].is_null(),
        "multi-key upload must finalize once the only chunk lands"
    );
}

#[actix_web::test]
async fn test_co_owner_uploads_into_shared_folder() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "co-owner-upload");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::CoOwner);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::CoOwner,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "co-owner-mk",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let _ = context;
}

#[actix_web::test]
async fn test_reader_upload_into_shared_folder_returns_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "reader-upload");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Reader);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "reader-mk",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "forbidden_not_writer");
    let _ = context;
}

#[actix_web::test]
async fn test_upload_multikey_missing_member_key_returns_409_share_membership_changed() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "missing-mk");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    // Omit alice from member_keys — server should 409.
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "missing-mk",
        vec![(bob.user_id, "wrap-b", true)],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body_text = test::read_body(resp).await;
    let body: Value = serde_json::from_slice(&body_text).unwrap();
    let inner: Value = serde_json::from_str(body["message"].as_str().unwrap()).unwrap();
    assert_eq!(inner["code"], "share_membership_changed");
    let _ = context;
}

#[actix_web::test]
async fn test_upload_multikey_extra_member_key_returns_409_share_membership_changed() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let folder = create_folder!(app, alice, "extra-mk");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    // Send a wrap for carol even though carol isn't on the folder.
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "extra-mk",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
            (carol.user_id, "wrap-c", false),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let _ = context;
}

#[actix_web::test]
async fn test_upload_multikey_stale_snapshot_returns_409() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "stale-snap");
    let now_share = now_secs();
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_share,
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    // Snapshot claims the membership signature is from before the
    // folder share was created — TOCTOU defense triggers.
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "stale",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
        ],
        now_share - 100,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let _ = context;
}

#[actix_web::test]
async fn test_upload_multikey_uploader_is_owner_of_new_file() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "owner-claim");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "owner-claim",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let bob_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(new_file_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .one(&context.db)
        .await
        .unwrap();
    assert!(bob_row.is_some(), "Bob (uploader) must own the new file row");
    let _ = context;
}

#[actix_web::test]
async fn test_upload_multikey_event_signature_invalid_returns_400() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "bad-sig");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let new_file_id = Uuid::new_v4();
    let timestamp = now_secs();
    // Sign over a *different* file_id so the verification fails.
    let event_signature = sign_no_recipient_event(
        &bob,
        Uuid::new_v4(),
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_upload_multikey_body(
        new_file_id,
        folder.id,
        "bad-sig",
        vec![
            (alice.user_id, "wrap-a", false),
            (bob.user_id, "wrap-b", true),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "event_signature_invalid");
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_adds_member_to_folder_with_own_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, yvonne, "yvonne@example.com");
    let folder = create_folder!(app, alice, "co-owner-adds");

    // Alice grants Bob Co-owner. Bob re-shares to Yvonne as Reader.
    let owner_plus_bob = owner_plus_recipient(&alice, &bob, ShareRoleEnum::CoOwner);
    grant_folder!(
        app,
        alice,
        bob,
        ShareRoleEnum::CoOwner,
        folder.id,
        &alice,
        &owner_plus_bob,
        alice
    );
    let members_after_yvonne = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &bob, share_role: ShareRoleEnum::CoOwner, is_owner: false, signed_by: &alice },
        FolderListMemberSpec { user: &yvonne, share_role: ShareRoleEnum::Reader, is_owner: false, signed_by: &bob },
    ];
    let envelope = build_co_owner_folder_share_envelope(
        &bob,
        &yvonne,
        ShareRoleEnum::Reader,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after_yvonne,
        &bob,
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Yvonne's user_files row records bob as the granter.
    let yvonne_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder.id))
        .filter(user_files::Column::UserId.eq(yvonne.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("yvonne row");
    assert_eq!(yvonne_row.shared_by_user_id, Some(bob.user_id));
    assert_eq!(yvonne_row.share_role, "reader");
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_revoke_cascades_their_grants_under_folder() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, yvonne, "yvonne@example.com");
    let folder = create_folder!(app, alice, "cascade-folder");

    let owner_plus_bob = owner_plus_recipient(&alice, &bob, ShareRoleEnum::CoOwner);
    grant_folder!(
        app,
        alice,
        bob,
        ShareRoleEnum::CoOwner,
        folder.id,
        &alice,
        &owner_plus_bob,
        alice
    );
    let members_after_yvonne = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &bob, share_role: ShareRoleEnum::CoOwner, is_owner: false, signed_by: &alice },
        FolderListMemberSpec { user: &yvonne, share_role: ShareRoleEnum::Reader, is_owner: false, signed_by: &bob },
    ];
    let envelope = build_co_owner_folder_share_envelope(
        &bob,
        &yvonne,
        ShareRoleEnum::Reader,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after_yvonne,
        &bob,
    );
    assert_eq!(post_share!(app, bob, envelope).status(), StatusCode::CREATED);

    // Alice revokes Bob → Yvonne's row must also go (cascade on
    // shared_by_user_id) in the same transaction.
    let after_revoke = vec![FolderListMemberSpec {
        user: &alice,
        share_role: ShareRoleEnum::CoOwner,
        is_owner: true,
        signed_by: &alice,
    }];
    let revoke = build_folder_revoke_body(
        &alice,
        &bob,
        folder.id,
        alice.user_id,
        ShareRoleEnum::CoOwner,
        now_secs(),
        &after_revoke,
        &alice,
    );
    let resp = delete_share!(app, alice, folder.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let yvonne_after = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder.id))
        .filter(user_files::Column::UserId.eq(yvonne.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(yvonne_after.is_none(), "yvonne's grant is cascade-revoked");

    let cascade_rows = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("shared_by_co_owner_revoked"))
        .all(&context.db)
        .await
        .unwrap();
    assert!(!cascade_rows.is_empty(), "cascade audit row recorded");
    let _ = context;
}

#[actix_web::test]
async fn test_member_list_endpoint_returns_signed_response() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "member-list");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/folder/{}/members", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["folder_id"], folder.id.to_string());
    assert_eq!(body["folder_owner_id"], alice.user_id.to_string());
    assert_eq!(body["signature_algorithm"], "rsa-pss-sha256");
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    let owner_member = members
        .iter()
        .find(|m| m["user_id"].as_str() == Some(&alice.user_id.to_string()))
        .unwrap();
    assert_eq!(owner_member["is_owner"], true);
    let _ = context;
}

#[actix_web::test]
async fn test_move_into_shared_folder_re_wraps_for_all_members() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "move-target");

    // Alice shares the folder with Bob as Editor.
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Alice has a private file she wants to move into the shared folder.
    let private_file = create_file!(app, alice, "alice-private-move");

    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        private_file.id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_move_into_shared_body(
        private_file.id,
        folder.id,
        vec![
            (alice.user_id, "rewrap-a"),
            (bob.user_id, "rewrap-b"),
        ],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Bob now has a user_files row on the moved file with the
    // re-wrapped key.
    let bob_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(private_file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob row on moved file");
    assert!(!bob_row.is_owner);
    assert_eq!(bob_row.encrypted_key, "rewrap-b");
    let _ = context;
}

#[actix_web::test]
async fn test_move_folder_into_shared_cascade_rewraps_all_descendants() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "cascade-dest");

    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Alice's private folder: root + two descendants (a child file and a
    // grandchild file under a sub-folder).
    let moved_root = create_folder!(app, alice, "alice-moved-root");
    let child_file = create_child_file!(app, alice, "alice-child", moved_root.id);
    let sub_folder = create_folder!(app, alice, "alice-sub", moved_root.id);
    let grandchild = create_child_file!(app, alice, "alice-grandchild", sub_folder.id);

    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        moved_root.id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    // One entry per node — every node re-wrapped for both alice (owner)
    // and bob (destination member).
    let entries = vec![
        (
            moved_root.id,
            vec![(alice.user_id, "wrap-root-a"), (bob.user_id, "wrap-root-b")],
        ),
        (
            child_file.id,
            vec![(alice.user_id, "wrap-child-a"), (bob.user_id, "wrap-child-b")],
        ),
        (
            sub_folder.id,
            vec![(alice.user_id, "wrap-sub-a"), (bob.user_id, "wrap-sub-b")],
        ),
        (
            grandchild.id,
            vec![(alice.user_id, "wrap-gc-a"), (bob.user_id, "wrap-gc-b")],
        ),
    ];
    let body = build_move_into_shared_cascade_body(
        moved_root.id,
        folder.id,
        entries,
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let resp_body = test::read_body(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "cascade move failed: {}",
        String::from_utf8_lossy(&resp_body)
    );

    // Bob has a row on every node with the supplied wrap.
    for (file_id, expected_wrap) in [
        (moved_root.id, "wrap-root-b"),
        (child_file.id, "wrap-child-b"),
        (sub_folder.id, "wrap-sub-b"),
        (grandchild.id, "wrap-gc-b"),
    ] {
        let bob_row = user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(file_id))
            .filter(user_files::Column::UserId.eq(bob.user_id))
            .one(&context.db)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("bob row missing on {file_id}"));
        assert!(!bob_row.is_owner);
        assert_eq!(bob_row.encrypted_key, expected_wrap);
        assert_eq!(bob_row.share_role, "editor");
    }

    // Root re-parented under the shared folder.
    let moved = entity::files::Entity::find_by_id(moved_root.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.file_id, Some(folder.id));

    // Exactly one move audit row for the root.
    let move_events = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("shared_folder_upload"))
        .filter(share_events::Column::FileId.eq(moved_root.id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(move_events.len(), 1);
    let _ = context;
}

#[actix_web::test]
async fn test_move_folder_into_shared_rejects_incomplete_subtree() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "incomplete-dest");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let moved_root = create_folder!(app, alice, "incomplete-root");
    let child_file = create_child_file!(app, alice, "incomplete-child", moved_root.id);

    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        moved_root.id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    // Omit the child node — server recomputes the subtree and rejects.
    let _ = child_file;
    let entries = vec![(
        moved_root.id,
        vec![(alice.user_id, "wrap-root-a"), (bob.user_id, "wrap-root-b")],
    )];
    let body = build_move_into_shared_cascade_body(
        moved_root.id,
        folder.id,
        entries,
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "entries_do_not_match_subtree");
    let _ = context;
}

#[actix_web::test]
async fn test_move_folder_into_shared_flat_shape_rejected() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "flat-dest");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // A folder posted with the flat single-file shape (no `entries`) must
    // be refused — re-parenting it would leave descendants encrypted for
    // alice alone (the latent E2E break S1 closes).
    let moved_folder = create_folder!(app, alice, "flat-folder");
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        moved_folder.id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_move_into_shared_body(
        moved_folder.id,
        folder.id,
        vec![(alice.user_id, "wrap-a"), (bob.user_id, "wrap-b")],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "move_folder_requires_cascade");
    let _ = context;
}

#[actix_web::test]
async fn test_move_into_shared_non_owner_rejected() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "non-owner-dest");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Bob (Editor on F, can_write) shares one of his OWN files with alice,
    // then alice — a non-owner of that file but writer on the dest — tries
    // to move it into F. Only the file's owner may move-into-shared.
    let bob_file = create_file!(app, bob, "bob-owned");
    grant!(app, bob, alice, ShareRoleEnum::Editor, bob_file.id);

    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        bob_file.id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    let body = build_move_into_shared_body(
        bob_file.id,
        folder.id,
        vec![(alice.user_id, "wrap-a"), (bob.user_id, "wrap-b")],
        timestamp,
        None,
        event_signature,
        timestamp,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    // `Error::Forbidden` maps to HTTP 401 in this codebase (see error::lib).
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_move_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_move_into_shared_rejects_own_subtree() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    let parent = create_folder!(app, alice, "cycle-parent");
    let child = create_folder!(app, alice, "cycle-child", parent.id);

    // Moving a folder into itself, or into one of its own descendants, would
    // create a parent cycle and make the recursive subtree walks loop without
    // bound. Both must be rejected before any re-parent happens. Caller owns
    // every node here, so it clears the writer/ownership gates and lands on
    // the cycle guard.
    for dest_id in [parent.id, child.id] {
        let timestamp = now_secs();
        let event_signature = sign_no_recipient_event(
            &alice,
            parent.id,
            AuditEventActionEnum::SharedFolderUpload,
            timestamp,
        );
        let body = build_move_into_shared_body(
            parent.id,
            dest_id,
            vec![(alice.user_id, "wrap")],
            timestamp,
            None,
            event_signature,
            timestamp,
        );
        let req = test::TestRequest::post()
            .uri("/api/storage/move-into-shared")
            .cookie(alice.jwt.clone())
            .set_json(&body)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["message"], "cannot_move_into_own_subtree");
    }
    let _ = context;
}

// Move out of a shared folder. The file owner detaches their own file
// from a share; every other member loses access.

#[actix_web::test]
async fn test_move_out_of_shared_drops_member_rows() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "moveout-dest");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Alice moves a private file into the shared folder first (bob gets a
    // row), then moves it back out — bob's row must be dropped.
    let private_file = create_file!(app, alice, "alice-moveout-file");
    let move_in_ts = now_secs();
    let move_in_sig = sign_no_recipient_event(
        &alice,
        private_file.id,
        AuditEventActionEnum::SharedFolderUpload,
        move_in_ts,
    );
    let move_in = build_move_into_shared_body(
        private_file.id,
        folder.id,
        vec![(alice.user_id, "rewrap-a"), (bob.user_id, "rewrap-b")],
        move_in_ts,
        None,
        move_in_sig,
        move_in_ts,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&move_in)
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    assert!(
        user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(private_file.id))
            .filter(user_files::Column::UserId.eq(bob.user_id))
            .one(&context.db)
            .await
            .unwrap()
            .is_some(),
        "bob has a row after move-in"
    );

    // Move out to alice's root.
    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &alice,
        private_file.id,
        AuditEventActionEnum::SharedFolderMoveOut,
        timestamp,
    );
    let body = build_move_out_of_shared_body(private_file.id, None, event_signature, timestamp);
    let req = test::TestRequest::post()
        .uri("/api/storage/move-out-of-shared")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let resp_body = test::read_body(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "move-out failed: {}",
        String::from_utf8_lossy(&resp_body)
    );

    // Bob's row is gone; alice still owns the file at root.
    let bob_after = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(private_file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(bob_after.is_none(), "bob loses access on move-out");
    let alice_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(private_file.id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("alice keeps her owner row");
    assert!(alice_row.is_owner);
    let moved = entity::files::Entity::find_by_id(private_file.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.file_id, None, "file back at alice's root");

    let move_out_events = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("shared_folder_move_out"))
        .filter(share_events::Column::FileId.eq(private_file.id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(move_out_events.len(), 1, "one move-out audit row");
    let _ = context;
}

#[actix_web::test]
async fn test_move_out_non_owner_blocked() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "moveout-block-dest");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Alice owns a file inside the shared folder; bob (Editor member) tries
    // to move it out. A non-owner has no move-out authority — only
    // self-remove. Server must 403.
    let private_file = create_file!(app, alice, "alice-file-in-folder");
    let move_in_ts = now_secs();
    let move_in_sig = sign_no_recipient_event(
        &alice,
        private_file.id,
        AuditEventActionEnum::SharedFolderUpload,
        move_in_ts,
    );
    let move_in = build_move_into_shared_body(
        private_file.id,
        folder.id,
        vec![(alice.user_id, "rewrap-a"), (bob.user_id, "rewrap-b")],
        move_in_ts,
        None,
        move_in_sig,
        move_in_ts,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/move-into-shared")
        .cookie(alice.jwt.clone())
        .set_json(&move_in)
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

    let timestamp = now_secs();
    let event_signature = sign_no_recipient_event(
        &bob,
        private_file.id,
        AuditEventActionEnum::SharedFolderMoveOut,
        timestamp,
    );
    let body = build_move_out_of_shared_body(private_file.id, None, event_signature, timestamp);
    let req = test::TestRequest::post()
        .uri("/api/storage/move-out-of-shared")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    // `Error::Forbidden` maps to HTTP 401 in this codebase (see error::lib).
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_move_not_owner");

    // Bob still has his row — the blocked move changed nothing.
    assert!(
        user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(private_file.id))
            .filter(user_files::Column::UserId.eq(bob.user_id))
            .one(&context.db)
            .await
            .unwrap()
            .is_some(),
        "blocked move-out left bob's row intact"
    );
    let _ = context;
}

// members_list_signature is a hard requirement.

#[actix_web::test]
async fn test_folder_share_requires_list_signature_on_create_400_if_missing() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "list-sig-missing");
    // build_share_envelope skips the list signature — server must 400.
    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        vec![(folder.id, b"wrap".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "missing_members_list_signature");
    let _ = context;
}

#[actix_web::test]
async fn test_folder_share_stores_list_signature_on_create() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "list-sig-store");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // GET /api/shares/folder/{F}/members returns the stamped signature.
    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/folder/{}/members", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert!(body["members_list_signature"].is_string());
    assert!(body["members_signed_at"].is_i64());
    assert_eq!(body["members_list_signed_by_user_id"], alice.user_id.to_string());
    let _ = context;
}

#[actix_web::test]
async fn test_folder_share_tampered_list_signature_rejected_400() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, eve, "eve@example.com");
    let folder = create_folder!(app, alice, "tampered-list");
    // Build the envelope for bob but sign the list as if eve were also
    // in it — the server reconstructs the post-mutation list from its
    // own DB and finds the bytes the client signed differ from the
    // canonical projection.
    let tampered_members = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &bob, share_role: ShareRoleEnum::Editor, is_owner: false, signed_by: &alice },
        FolderListMemberSpec { user: &eve, share_role: ShareRoleEnum::Reader, is_owner: false, signed_by: &alice },
    ];
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &tampered_members,
        &alice,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "members_list_signature_invalid");
    let _ = context;
}

#[actix_web::test]
async fn test_folder_share_signer_not_authorized_returns_400() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, eve, "eve@example.com");
    let folder = create_folder!(app, alice, "unauthorized-signer");
    // Eve isn't on the folder share at all; her sig must be rejected.
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &eve,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "members_list_signer_not_authorized");
    let _ = context;
}

#[actix_web::test]
async fn test_folder_revoke_requires_list_signature_400_if_missing() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "revoke-no-sig");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // build_revoke_body omits the list signature; folder revoke must 400.
    let revoke = build_revoke_body(&alice, &bob, folder.id, ShareRoleEnum::Editor, now_secs());
    let resp = delete_share!(app, alice, folder.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "missing_members_list_signature");
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_reshare_signs_list_with_co_owner_key() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let folder = create_folder!(app, alice, "co-owner-resign");
    // Alice grants Bob Co-owner first.
    let alice_plus_bob = owner_plus_recipient(&alice, &bob, ShareRoleEnum::CoOwner);
    grant_folder!(
        app,
        alice,
        bob,
        ShareRoleEnum::CoOwner,
        folder.id,
        &alice,
        &alice_plus_bob,
        alice
    );
    // Bob (Co-owner) re-shares to Carol. Bob signs the new list.
    let members_with_carol = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &bob, share_role: ShareRoleEnum::CoOwner, is_owner: false, signed_by: &alice },
        FolderListMemberSpec { user: &carol, share_role: ShareRoleEnum::Reader, is_owner: false, signed_by: &bob },
    ];
    let envelope = build_co_owner_folder_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_with_carol,
        &bob,
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Server stored the list signature attributed to Bob.
    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/folder/{}/members", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["members_list_signed_by_user_id"], bob.user_id.to_string());
    let _ = context;
}

#[actix_web::test]
async fn test_role_change_on_folder_requires_fresh_list_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "role-change");
    // Initial share: bob = editor.
    let members_editor = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_editor,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // Upgrade bob to co-owner: server requires a fresh list sig over
    // the new role AND an audit signature over the `role_change`
    // canonical (action + previous_role + new_role + timestamp).
    let members_coowner = owner_plus_recipient(&alice, &bob, ShareRoleEnum::CoOwner);
    let envelope = build_folder_role_change_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        ShareRoleEnum::CoOwner,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_coowner,
        &alice,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let _ = context;
}

// Member roster transparency: every member sees every member's email.

#[actix_web::test]
async fn test_folder_share_create_default_exposes_emails_to_every_member() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "default-emails-visible");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/folder/{}/members", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    for member in body["members"].as_array().unwrap() {
        assert!(
            !member["email"].is_null(),
            "every member row carries an email"
        );
    }
    let _ = context;
}

#[actix_web::test]
async fn test_folder_owner_sees_full_member_roster() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "owner-roster");
    let members_after = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Editor);
    let envelope = build_folder_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/folder/{}/members", folder.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let members = body["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    for member in members {
        assert!(!member["email"].is_null());
    }
    let _ = context;
}
