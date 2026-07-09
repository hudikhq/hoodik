//! Grant/revoke/list roundtrip across the three roles, plus the
//! Co-owner cascade revoke and the recipient-list/listing endpoints.
//!
//! Each test boots a fresh mock server, registers 2-3 accounts via the
//! real auth route, creates a file via the real storage route, and drives
//! the share routes end-to-end. No DB shortcuts.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use entity::{links, share_events, user_files, users, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use serde_json::Value;
use shares::data::app_share::AppShare;
use shares::data::incoming::IncomingSharePage;

use crate::shares_common::*;

#[actix_web::test]
async fn test_owner_grants_reader_to_recipient_then_revokes() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "alice-file-1");

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-for-bob".to_vec())],
        random_nonce(),
        now_secs(),
    );

    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(resp).await;
    let shares: Vec<AppShare> = serde_json::from_value(body["shares"].clone()).unwrap();
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].share_role, "reader");
    assert_eq!(shares[0].recipient_id, bob.user_id);

    let revoke_body = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke_body);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let remaining = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(remaining.is_empty(), "bob's row should be gone after revoke");
}

#[actix_web::test]
async fn test_owner_grants_editor_to_recipient() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "editor-file");

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["shares"][0]["share_role"], "editor");
    let _ = context;
}

#[actix_web::test]
async fn test_owner_grants_co_owner_to_recipient() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "co-owner-file");

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::CoOwner,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["shares"][0]["share_role"], "co-owner");
    let _ = context;
}

#[actix_web::test]
async fn test_owner_can_share_with_unverified_recipient() {
    // Self-hosted deployments may run with
    // settings.users.enforce_email_activation = false, so unverified
    // accounts are legitimate users. Sharing must accept them as
    // recipients — login is what decides whether they can read.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "alice-shares-with-unverified");

    users::Entity::update(users::ActiveModel {
        id: ActiveValue::Unchanged(bob.user_id),
        email_verified_at: ActiveValue::Set(None),
        ..Default::default()
    })
    .exec(&context.db)
    .await
    .unwrap();

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(row.is_some(), "unverified recipient should have a user_files row after share");
}

#[actix_web::test]
async fn test_co_owner_can_reshare_at_reader_role() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "ladder-reader");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

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
    assert_eq!(resp.status(), StatusCode::CREATED);
    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(carol.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("carol's row should exist");
    assert_eq!(row.share_role, "reader");
    assert_eq!(row.shared_by_user_id, Some(bob.user_id));
}

#[actix_web::test]
async fn test_co_owner_can_reshare_at_editor_role() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "ladder-editor");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Editor,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["shares"][0]["share_role"], "editor");
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_cannot_reshare_at_co_owner_role_returns_403() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "ladder-coowner-deny");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::CoOwner,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_grant_equal_role");
    let _ = context;
}

#[actix_web::test]
async fn test_owner_revoke_co_owner_cascades_their_grants() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "cascade-target");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    // Bob, now a Co-owner, re-shares the file with Carol at Reader.
    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-for-carol".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Alice revokes Bob — Carol's row must vanish in the same transaction.
    let revoke = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::CoOwner, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let remaining = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert!(remaining.is_empty(), "all non-owner rows should be gone");

    let cascade_rows = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("shared_by_co_owner_revoked"))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(cascade_rows.len(), 1, "one cascade audit row for Carol");
    let cascade = &cascade_rows[0];
    assert!(cascade.sender_id.is_none(), "cascade is system-attributed");
    assert!(
        cascade.sender_signature.is_none(),
        "cascade rows have NULL sender_signature"
    );
    assert_eq!(cascade.recipient_id, Some(carol.user_id));
}

#[actix_web::test]
async fn test_links_index_is_owner_only() {
    // Public links are entirely owner-side — recipients on a shared file
    // never see the owner's link via `GET /api/links`, because the link
    // key wraps under the owner's pubkey only and surfacing the row to a
    // recipient delivers no actionable information.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "owner-only-link");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let link_id = entity::Uuid::new_v4();
    links::Entity::insert(links::ActiveModel {
        id: ActiveValue::Set(link_id),
        user_id: ActiveValue::Set(alice.user_id),
        file_id: ActiveValue::Set(file.id),
        signature: ActiveValue::Set("test-sig".to_string()),
        downloads: ActiveValue::Set(0),
        encrypted_name: ActiveValue::Set("encrypted-name".to_string()),
        encrypted_link_key: ActiveValue::Set("encrypted-link-key".to_string()),
        encrypted_thumbnail: ActiveValue::Set(None),
        encrypted_file_key: ActiveValue::Set(Some("encrypted-file-key".to_string())),
        created_at: ActiveValue::Set(now_secs()),
        expires_at: ActiveValue::Set(None),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();

    // Alice (owner) sees her link.
    let req = test::TestRequest::get()
        .uri("/api/links")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let array = body.as_array().expect("links list array");
    assert_eq!(array.len(), 1, "owner should see their own link");
    assert_eq!(array[0]["id"], link_id.to_string());

    // Bob (recipient on the file) gets an empty list — the recipient
    // never sees the owner's public link.
    let req = test::TestRequest::get()
        .uri("/api/links")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "recipient should not see the owner's link"
    );

    // Carol (no relation to the file) also sees an empty list.
    let req = test::TestRequest::get()
        .uri("/api/links")
        .cookie(carol.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[actix_web::test]
async fn test_recipient_cannot_revoke_owner_link() {
    // The relax only opens the LIST view to recipients; mutating routes
    // (create / revoke / update) stay owner-gated so a recipient cannot
    // delete a link the owner minted.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "recipient-cannot-revoke");
    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let link_id = entity::Uuid::new_v4();
    links::Entity::insert(links::ActiveModel {
        id: ActiveValue::Set(link_id),
        user_id: ActiveValue::Set(alice.user_id),
        file_id: ActiveValue::Set(file.id),
        signature: ActiveValue::Set("test-sig".to_string()),
        downloads: ActiveValue::Set(0),
        encrypted_name: ActiveValue::Set("encrypted-name".to_string()),
        encrypted_link_key: ActiveValue::Set("encrypted-link-key".to_string()),
        encrypted_thumbnail: ActiveValue::Set(None),
        encrypted_file_key: ActiveValue::Set(Some("encrypted-file-key".to_string())),
        created_at: ActiveValue::Set(now_secs()),
        expires_at: ActiveValue::Set(None),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/api/links/{}", link_id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_create_share_persists_supplied_member_signature() {
    // When the producer supplies `member_signature`
    // alongside the envelope, the server verifies it against the granter
    // and persists the raw bytes in `user_files.member_signature`.
    // Subsequent `verifyFolderMemberList` calls in the SPA can chain
    // trust from the owner through the named signer to the recipient.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "member-sig-roundtrip");

    let timestamp = now_secs();
    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-for-bob".to_vec())],
        random_nonce(),
        timestamp,
    );
    let envelope = inject_member_signature(
        envelope,
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        timestamp,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob's row should exist");
    assert!(
        row.member_signature.is_some(),
        "member_signature must persist verbatim"
    );
    assert!(
        !row.member_signature.as_ref().unwrap().is_empty(),
        "stored sig bytes are non-empty"
    );
}

#[actix_web::test]
async fn test_create_share_rejects_member_signature_signed_by_stranger() {
    // The σ commits to the granter via the verifier's pubkey choice.
    // Forging the signature with a third party's privkey while
    // claiming to be Alice must fail at the server boundary.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, eve, "eve@example.com");
    let file = create_file!(app, alice, "member-sig-stranger");

    let timestamp = now_secs();
    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        timestamp,
    );
    // Sign with Eve's key but submit through Alice's session.
    let envelope = inject_member_signature(
        envelope,
        &eve,
        &bob,
        ShareRoleEnum::Reader,
        timestamp,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "member_signature_invalid");
}

#[actix_web::test]
async fn test_co_owner_reshare_persists_co_owner_signed_member_signature() {
    // Bob is anointed Co-owner by Alice; Bob reshares to Carol with
    // a fresh σ. The persisted row records Bob's σ — exactly the
    // missing producer behaviour.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "co-owner-resharer-member-sig");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    let timestamp = now_secs();
    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-by-bob".to_vec())],
        random_nonce(),
        timestamp,
    );
    let envelope = inject_member_signature(
        envelope,
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        timestamp,
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(carol.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("carol's row should exist");
    assert_eq!(row.shared_by_user_id, Some(bob.user_id));
    assert!(
        row.member_signature.is_some(),
        "Co-owner reshare must persist σ verbatim"
    );
}

#[actix_web::test]
async fn test_create_share_accepts_legacy_envelope_without_member_signature() {
    // Legacy clients ship envelopes without σ. The server treats them
    // as legacy, persists `member_signature = None`,
    // and lets the SPA fall back to trusting the owner only via
    // `verifyFolderMemberList`'s legacy path. Don't break old clients.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "legacy-member-sig-fallback");

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob's row should exist");
    assert!(row.member_signature.is_none());
}

#[actix_web::test]
async fn test_co_owner_can_revoke_recipient_granted_by_owner() {
    // A Co-owner has full peer rights with the owner over
    // the share — they can ADD, CHANGE role, and REMOVE any recipient
    // (other than the file owner). Previously this was gated on the
    // Co-owner being the original grantor.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "co-owner-revokes-others");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file.id);

    let revoke = build_revoke_body(&bob, &carol, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, bob, file.id, carol.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let remaining = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(carol.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(remaining.is_empty(), "carol's row should be gone after co-owner revoke");
}

#[actix_web::test]
async fn test_editor_still_cannot_revoke_peer_returns_403() {
    // Editors gained read-only access to the member list in 79698bc but
    // never gained mutation rights — `can_reshare()` is false. This pins
    // the bound after the Co-owner relax.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "editor-cannot-revoke");

    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file.id);

    let revoke = build_revoke_body(&bob, &carol, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, bob, file.id, carol.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "cannot_revoke_not_owner");
    let _ = context;
}

#[actix_web::test]
async fn test_co_owner_change_role_via_create_share_accepted() {
    // Role-change is the create_share path with the same recipient and
    // a new role. Co-owners are already permitted to call create_share;
    // this pins the contract end-to-end so a future tightening of the
    // create-share gate doesn't silently take away the change affordance.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "co-owner-changes-role");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file.id);

    // The Co-owner upgrading Carol from Reader to Editor is a role_change
    // path — the envelope's `event_signature` must cover the role_change
    // canonical (action + before + after), not the legacy `shared_by_co_owner`
    // canonical the helper produces for a fresh re-share.
    let envelope = build_role_change_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        ShareRoleEnum::Editor,
        file.id,
        vec![(file.id, b"wrapped-rotation".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(carol.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("carol's row should exist");
    assert_eq!(row.share_role, "editor");
}

#[actix_web::test]
async fn test_get_mine_returns_only_incoming_shares() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "mine-file");
    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let page: IncomingSharePage = test::read_body_json(resp).await;
    assert_eq!(page.total, 1);
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].file_id, file.id);
    assert_eq!(page.items[0].owner_id, alice.user_id);
    assert_eq!(page.items[0].share_role, "editor");

    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let alice_page: IncomingSharePage = test::read_body_json(resp).await;
    assert_eq!(
        alice_page.total, 0,
        "alice should not see her own outgoing shares"
    );
    let _ = context;
}

#[actix_web::test]
async fn test_get_mine_carries_file_size_and_upload_progress() {
    // The recipient-side DetailsModal + TableFileRow read size and
    // upload-progress fields straight off the IncomingShare. Without
    // them the UI renders "Size: 0 B" + a never-finishing progress
    // chip on a shared file.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "metadata-file");

    // The file row gets `finished_upload_at` + `chunks_stored` only
    // after the last chunk lands; create_file alone doesn't stamp it.
    // Patch the row directly to mirror a finished upload.
    let finished_at = 1_700_000_999i64;
    entity::files::Entity::update(entity::files::ActiveModel {
        id: ActiveValue::Set(file.id),
        chunks_stored: ActiveValue::Set(Some(1)),
        finished_upload_at: ActiveValue::Set(Some(finished_at)),
        ..Default::default()
    })
    .exec(&context.db)
    .await
    .expect("stamp finished upload on file row");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let page: IncomingSharePage = test::read_body_json(resp).await;
    let row = page
        .items
        .iter()
        .find(|i| i.file_id == file.id)
        .expect("incoming list must contain the shared file");

    assert_eq!(row.size, Some(1024));
    assert_eq!(row.chunks, Some(1));
    assert_eq!(row.chunks_stored, Some(1));
    assert_eq!(row.finished_upload_at, Some(finished_at));
    assert_eq!(row.md5.as_deref(), Some("md5"));
    assert_eq!(row.sha1.as_deref(), Some("sha1"));
    assert_eq!(row.sha256.as_deref(), Some("sha256"));
    assert_eq!(row.blake2b.as_deref(), Some("b2b"));
}

#[actix_web::test]
async fn test_get_mine_returns_only_roots_not_descendants() {
    // Sharing a folder eagerly creates `user_files` rows for the
    // recipient on every descendant so each file can
    // carry an RSA-wrapped key. The recipient-facing list must dedupe
    // back to the roots of each share, otherwise the synthetic "Shared
    // with me" view flattens the tree — a 3-file folder share would
    // show the folder *and* its three children at the same level.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "root-folder");
    let child_one = create_child_file!(app, alice, "child-one", folder.id);
    let child_two = create_child_file!(app, alice, "child-two", folder.id);

    let standalone = create_file!(app, alice, "standalone");

    let members = vec![
        crate::shares_common::FolderListMemberSpec {
            user: &alice,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: &alice,
        },
        crate::shares_common::FolderListMemberSpec {
            user: &bob,
            share_role: ShareRoleEnum::Editor,
            is_owner: false,
            signed_by: &alice,
        },
    ];
    let entries = vec![
        (folder.id, b"wrap-folder".to_vec()),
        (child_one.id, b"wrap-c1".to_vec()),
        (child_two.id, b"wrap-c2".to_vec()),
    ];
    let envelope = crate::shares_common::build_folder_share_envelope_with_entries(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        entries,
        crate::shares_common::random_nonce(),
        crate::shares_common::now_secs(),
        &members,
        &alice,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    grant!(app, alice, bob, ShareRoleEnum::Reader, standalone.id);

    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let page: IncomingSharePage = test::read_body_json(resp).await;
    let file_ids: Vec<entity::Uuid> = page.items.iter().map(|i| i.file_id).collect();
    assert_eq!(page.total, 2, "list should only carry the two roots");
    assert!(file_ids.contains(&folder.id), "folder root present");
    assert!(file_ids.contains(&standalone.id), "standalone file present");
    assert!(!file_ids.contains(&child_one.id), "descendants must not flatten");

    // Bob still has a `user_files` row for the descendant — needed for
    // navigation + key fan-out — it just doesn't surface at the root.
    let row = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::FileId.eq(child_one.id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(row.is_some());
}

#[actix_web::test]
async fn test_get_mine_by_user_filters_by_sender() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");

    let file_a = create_file!(app, alice, "filter-a");
    let file_b = create_file!(app, bob, "filter-b");

    grant!(app, alice, carol, ShareRoleEnum::Reader, file_a.id);
    grant!(app, bob, carol, ShareRoleEnum::Reader, file_b.id);

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/mine/by/{}", alice.user_id))
        .cookie(carol.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let page: IncomingSharePage = test::read_body_json(resp).await;
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].file_id, file_a.id);
    assert_eq!(page.items[0].shared_by_user_id, Some(alice.user_id));
    let _ = context;
}

#[actix_web::test]
async fn test_get_recipient_list_visible_to_every_member_blocked_for_strangers() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    register_user!(app, context, dave, "dave@example.com");
    let file = create_file!(app, alice, "shared-roster");
    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file.id);

    for (label, user) in [("owner", &alice), ("editor", &bob), ("reader", &carol)] {
        let req = test::TestRequest::get()
            .uri(&format!("/api/shares/{}", file.id))
            .cookie(user.jwt.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "{label} must see the full member list"
        );
        let shares: Vec<AppShare> = test::read_body_json(resp).await;
        let recipients: std::collections::HashSet<_> =
            shares.iter().map(|s| s.recipient_id).collect();
        assert!(
            recipients.contains(&bob.user_id) && recipients.contains(&carol.user_id),
            "{label} sees every recipient row"
        );
        assert!(
            shares.iter().all(|s| !s.recipient_email.is_empty()),
            "{label} sees recipient emails on every row"
        );
    }

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/{}", file.id))
        .cookie(dave.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "non-member still receives 404"
    );
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "file_not_found");
    let _ = context;
}

#[actix_web::test]
async fn test_recipient_self_removes_from_shared_file() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "self-remove-target");
    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let revoke = build_revoke_body(&bob, &bob, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, bob, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let bob_rows = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(bob_rows.is_empty(), "bob's user_files row is gone after self-remove");

    let alice_owner_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .one(&context.db)
        .await
        .unwrap();
    assert!(alice_owner_row.is_some(), "alice still owns the file");

    let events = share_events::Entity::find()
        .filter(share_events::Column::FileId.eq(file.id))
        .filter(share_events::Column::Action.eq("revoke"))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sender_id, Some(bob.user_id));
    assert_eq!(events[0].recipient_id, Some(bob.user_id));
    assert!(events[0].sender_signature.is_some(), "self-remove is signed by the recipient");
}

#[actix_web::test]
async fn test_co_owner_self_remove_cascades_their_grants() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "self-remove-cascade");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);

    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-for-carol".to_vec())],
        random_nonce(),
        now_secs(),
    );
    assert_eq!(post_share!(app, bob, envelope).status(), StatusCode::CREATED);

    let revoke = build_revoke_body(&bob, &bob, file.id, ShareRoleEnum::CoOwner, now_secs());
    let resp = delete_share!(app, bob, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let remaining = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert!(remaining.is_empty(), "bob's row and carol's downstream row both gone");

    let cascade = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("shared_by_co_owner_revoked"))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(cascade.len(), 1);
    assert_eq!(cascade[0].recipient_id, Some(carol.user_id));
    assert!(cascade[0].sender_id.is_none(), "cascade is system-attributed");
}

#[actix_web::test]
async fn test_recipient_self_removes_from_shared_folder_without_list_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "self-remove-folder");
    let child = create_child_file!(app, alice, "self-remove-child", folder.id);

    let members_after = vec![
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
            (folder.id, b"wrap-folder".to_vec()),
            (child.id, b"wrap-child".to_vec()),
        ],
        random_nonce(),
        now_secs(),
        &members_after,
        &alice,
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    // The leaving recipient can't sign the post-mutation roster, so the
    // revoke body omits members_list_signature. Folder self-remove must
    // succeed regardless.
    let revoke = build_revoke_body(&bob, &bob, folder.id, ShareRoleEnum::Editor, now_secs());
    let resp = delete_share!(app, bob, folder.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let bob_rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        bob_rows.is_empty(),
        "bob's rows on the folder and the child are gone"
    );

    let alice_rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .all(&context.db)
        .await
        .unwrap();
    assert!(alice_rows.len() >= 2, "alice still owns folder + child");
}

#[actix_web::test]
async fn test_storage_index_rejects_synthetic_shared_with_me_id() {
    // The SPA injects a client-only `__shared_with_me__` parent for the
    // virtual folder; that string is never a UUID, so reaching the storage
    // list endpoint with it used to surface as a 500 (uuid parse) — the
    // worst kind of bug class for a known-bad input. Backend short-
    // circuits to a 400 with a clear code so a future frontend regression
    // is visible in the network log instead of looking like a server crash.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let req = test::TestRequest::get()
        .uri("/api/storage?dir_id=__shared_with_me__")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "invalid_dir_id");
    let _ = context;
}
