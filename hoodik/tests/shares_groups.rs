//! Share-group CRUD + roster tests, plus the move-out / revoke fixes a
//! group rework depends on.
//!
//! A group is a saved recipient selection — owner plus members — with no
//! file associations of its own. The server only does group CRUD and
//! roster mutations; sharing to a group is a client-side fan-out of
//! ordinary per-person shares. These tests boot a fresh mock server,
//! register participants via the real auth route, and assert against the
//! `share_group_members` / `user_files` / `share_events` / `files` tables.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::{AuditEventActionEnum, ShareRoleEnum};
use entity::{
    files, share_group_members, share_groups, user_files, users, ActiveValue, ColumnTrait,
    EntityTrait, QueryFilter,
};
use hoodik::server;
use serde_json::{json, Value};
use shares::data::group::{AppShareGroup, GroupsResponse};

use crate::shares_common::*;

/// Drive `POST /api/shares/groups` and parse the response. Inlined as
/// a macro because actix-web's `init_service` produces a complex Service
/// type that doesn't survive a simple `impl Service<...>` bound.
macro_rules! create_group {
    ($app:expr, $user:expr, $name:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri("/api/shares/groups")
            .cookie($user.jwt.clone())
            .set_json(&serde_json::json!({ "name": $name }))
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<shares::data::group::AppShareGroup>(&body)
            .expect("create_group json")
    }};
}

/// Body for `POST /api/shares/groups/{id}/members` in the new model: a
/// plain roster insert guarded by the replay nonce + timestamp. No file
/// keys, no signatures, no cascade.
fn add_member_body(member: &TestUser, group_role: &str) -> Value {
    json!({
        "user_id": member.user_id.to_string(),
        "pubkey_fingerprint": member.fingerprint,
        "group_role": group_role,
        "timestamp": now_secs(),
        "nonce": cryptfns::base64::encode(random_nonce()),
    })
}

/// Seed a `share_group_members` row directly at `group_role`. Fixture
/// members are seeded past the route so each test exercises the route only
/// for the participant under test.
async fn seed_member_directly(
    context: &context::Context,
    group_id: entity::Uuid,
    user_id: entity::Uuid,
    group_role: &str,
) {
    share_group_members::Entity::insert(share_group_members::ActiveModel {
        group_id: ActiveValue::Set(group_id),
        user_id: ActiveValue::Set(user_id),
        added_at: ActiveValue::Set(now_secs()),
        role: ActiveValue::Set(group_role.to_string()),
    })
    .exec_without_returning(&context.db)
    .await
    .expect("seed group member");
}

#[actix_web::test]
async fn test_create_group_returns_201_with_unique_name_constraint() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");

    let req = test::TestRequest::post()
        .uri("/api/shares/groups")
        .cookie(alice.jwt.clone())
        .set_json(json!({"name": "Family"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let group: AppShareGroup = test::read_body_json(resp).await;
    assert_eq!(group.name, "Family");
    assert_eq!(group.owner_id, alice.user_id);

    let row = share_groups::Entity::find_by_id(group.id)
        .one(&context.db)
        .await
        .unwrap()
        .expect("group row persisted");
    assert_eq!(row.name, "Family");
}

#[actix_web::test]
async fn test_create_group_duplicate_name_returns_409() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");

    let req = test::TestRequest::post()
        .uri("/api/shares/groups")
        .cookie(alice.jwt.clone())
        .set_json(json!({"name": "Buddies"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/api/shares/groups")
        .cookie(alice.jwt.clone())
        .set_json(json!({"name": "Buddies"}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "group_name_taken");
    let _ = context;
}

#[actix_web::test]
async fn test_delete_group_drops_members_via_cascade() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_user!(app, carol, "carol@example.com");

    let group = create_group!(app, alice, "Friends");
    seed_member_directly(&context, group.id, bob.user_id, "reader").await;
    seed_member_directly(&context, group.id, carol.user_id, "reader").await;

    let req = test::TestRequest::delete()
        .uri(&format!("/api/shares/groups/{}", group.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let remaining_members = share_group_members::Entity::find()
        .filter(share_group_members::Column::GroupId.eq(group.id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        remaining_members.is_empty(),
        "FK cascade should clear share_group_members on group delete"
    );
    let remaining_groups = share_groups::Entity::find_by_id(group.id)
        .one(&context.db)
        .await
        .unwrap();
    assert!(remaining_groups.is_none());
}

#[actix_web::test]
async fn test_non_member_cannot_touch_group() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let group = create_group!(app, alice, "Private");

    // Bob has no membership in Alice's group. Every group write resolves
    // his role as `None`; the routes hide the group identity by returning
    // 404 rather than 403 so the surface can't be used to probe ids
    // someone else created.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/shares/groups/{}", group.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(bob.jwt.clone())
        .set_json(&add_member_body(&alice, "reader"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/shares/groups/{}/members/{}",
            group.id, alice.user_id
        ))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Renaming someone else's group is equally invisible.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/shares/groups/{}", group.id))
        .cookie(bob.jwt.clone())
        .set_json(&json!({ "name": "Hijacked" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let _ = context;
}

#[actix_web::test]
async fn test_add_member_is_plain_roster_insert_with_no_file_grants() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // Alice owns file F and a group. Adding Bob to the group is a pure
    // roster write — Bob receives a membership row and NOTHING else; no
    // user_files row on any of Alice's files materialises from the add.
    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let file = create_file!(app, alice, "owner-only-file");
    let group = create_group!(app, alice, "Studio");

    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(alice.jwt.clone())
        .set_json(&add_member_body(&bob, "editor"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let membership = share_group_members::Entity::find_by_id((group.id, bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("Bob must be a group member after the add");
    assert_eq!(membership.role, "editor");

    let bob_rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        bob_rows.is_empty(),
        "adding a member must not grant any files — Bob holds no user_files row"
    );
    let _ = file;
}

#[actix_web::test]
async fn test_add_member_rejects_nonce_reuse() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_user!(app, carol, "carol@example.com");

    let group = create_group!(app, alice, "Studio");

    // The same body replayed verbatim trips the per-(caller, nonce) guard.
    let nonce = cryptfns::base64::encode(random_nonce());
    let timestamp = now_secs();
    let body = json!({
        "user_id": bob.user_id.to_string(),
        "pubkey_fingerprint": bob.fingerprint,
        "group_role": "reader",
        "timestamp": timestamp,
        "nonce": nonce,
    });

    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::NO_CONTENT);

    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["message"], "replay_nonce_seen");
    let _ = carol;
}

#[actix_web::test]
async fn test_remove_member_from_group_does_not_revoke_existing_user_files() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    // Bob has a pre-existing user_files row on F from an unrelated direct
    // share. Removing him from a group must not retroactively strip it —
    // a group is a recipient list, not a live ACL.
    let file = create_file!(app, alice, "group-no-revoke");
    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let group = create_group!(app, alice, "Squad");
    seed_member_directly(&context, group.id, bob.user_id, "reader").await;

    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/shares/groups/{}/members/{}",
            group.id, bob.user_id
        ))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let still_in_files = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(
        still_in_files.is_some(),
        "Bob's existing user_files row must survive group removal"
    );

    let no_longer_member = share_group_members::Entity::find_by_id((group.id, bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(no_longer_member.is_none());
}

#[actix_web::test]
async fn test_list_groups_returns_owned_and_member_of_sections() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let alice_group = create_group!(app, alice, "AliceTeam");
    let bob_group = create_group!(app, bob, "BobTeam");
    seed_member_directly(&context, bob_group.id, alice.user_id, "editor").await;

    let req = test::TestRequest::get()
        .uri("/api/shares/groups")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let response: GroupsResponse = test::read_body_json(resp).await;

    assert_eq!(response.owned.len(), 1);
    assert_eq!(response.owned[0].id, alice_group.id);
    assert_eq!(response.member_of.len(), 1);
    assert_eq!(response.member_of[0].id, bob_group.id);
    assert_eq!(response.member_of[0].owner_email, "bob@example.com");
    assert_eq!(response.member_of[0].group_role, "editor");
}

#[actix_web::test]
async fn test_group_members_returns_owner_and_members_with_keys() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // The roster the client fans a share out to is the owner plus every
    // member, each carrying the pubkey + fingerprint needed to wrap a file
    // key — the owner is a valid recipient when a member initiates a share.
    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_user!(app, carol, "carol@example.com");

    let group = create_group!(app, alice, "Studio");
    seed_member_directly(&context, group.id, bob.user_id, "editor").await;
    seed_member_directly(&context, group.id, carol.user_id, "reader").await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let roster: Vec<Value> = test::read_body_json(resp).await;

    assert_eq!(roster.len(), 3, "roster is owner + two members");
    let by_id: std::collections::HashMap<String, &Value> = roster
        .iter()
        .map(|m| (m["user_id"].as_str().unwrap().to_string(), m))
        .collect();

    let owner = by_id
        .get(&alice.user_id.to_string())
        .expect("owner present in roster");
    assert_eq!(owner["group_role"], "owner");
    assert_eq!(owner["email"], "alice@example.com");
    assert_eq!(owner["pubkey"], alice.public_pem);
    assert_eq!(owner["fingerprint"], alice.fingerprint);

    let bob_entry = by_id.get(&bob.user_id.to_string()).expect("bob present");
    assert_eq!(bob_entry["group_role"], "editor");
    assert_eq!(bob_entry["pubkey"], bob.public_pem);
    assert_eq!(bob_entry["fingerprint"], bob.fingerprint);

    let carol_entry = by_id.get(&carol.user_id.to_string()).expect("carol present");
    assert_eq!(carol_entry["group_role"], "reader");

    // A non-member cannot read the roster.
    register_user!(app, mallory, "mallory@example.com");
    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(mallory.jwt.clone())
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::NOT_FOUND
    );
}

#[actix_web::test]
async fn test_add_unverified_user_to_group_succeeds() {
    // Unverified accounts are legitimate users when
    // settings.users.enforce_email_activation is false, so the group
    // membership path must accept them too.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, ghost, "ghost@example.com");
    let group = create_group!(app, alice, "Inner Circle");

    users::Entity::update(users::ActiveModel {
        id: ActiveValue::Unchanged(ghost.user_id),
        email_verified_at: ActiveValue::Set(None),
        ..Default::default()
    })
    .exec(&context.db)
    .await
    .unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(alice.jwt.clone())
        .set_json(&add_member_body(&ghost, "reader"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let membership = share_group_members::Entity::find_by_id((group.id, ghost.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(membership.is_some(), "unverified user should be added to the group");
}

#[actix_web::test]
async fn test_co_owner_manages_roster_but_cannot_delete_or_grant_co_owner() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // Bob is a group co-owner. He may add members and set a member's role
    // up to editor — but he may NOT rename or delete the group (both
    // owner-only) nor mint another co-owner (privilege-escalation guard).
    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_user!(app, carol, "carol@example.com");
    register_user!(app, dan, "dan@example.com");

    let group = create_group!(app, alice, "Studio");
    seed_member_directly(&context, group.id, bob.user_id, "co-owner").await;
    seed_member_directly(&context, group.id, carol.user_id, "reader").await;

    // Co-owner adds Dan.
    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(bob.jwt.clone())
        .set_json(&add_member_body(&dan, "reader"))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::NO_CONTENT,
        "co-owner may add a member"
    );

    // Co-owner cannot rename the group — rename is owner-only.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/shares/groups/{}", group.id))
        .cookie(bob.jwt.clone())
        .set_json(&json!({ "name": "Renamed By Co-owner" }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::UNAUTHORIZED,
        "co-owner may not rename — owner-only"
    );

    // Co-owner sets Carol reader → editor.
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/shares/groups/{}/members/{}/role",
            group.id, carol.user_id
        ))
        .cookie(bob.jwt.clone())
        .set_json(&json!({ "group_role": "editor" }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::NO_CONTENT,
        "co-owner may set a lesser member to editor"
    );
    let carol_role = share_group_members::Entity::find_by_id((group.id, carol.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("carol still a member")
        .role;
    assert_eq!(carol_role, "editor");

    // Co-owner CANNOT promote Carol to co-owner.
    let req = test::TestRequest::put()
        .uri(&format!(
            "/api/shares/groups/{}/members/{}/role",
            group.id, carol.user_id
        ))
        .cookie(bob.jwt.clone())
        .set_json(&json!({ "group_role": "co-owner" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["message"], "cannot_set_role");

    // Co-owner CANNOT add a new member as co-owner either.
    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(bob.jwt.clone())
        .set_json(&add_member_body(&carol, "co-owner"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["message"], "cannot_grant_equal_role");

    // A group reader cannot manage the roster at all.
    let req = test::TestRequest::post()
        .uri(&format!("/api/shares/groups/{}/members", group.id))
        .cookie(carol.jwt.clone())
        .set_json(&add_member_body(&dan, "reader"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["message"], "not_group_manager");

    // Co-owner CANNOT delete the group — owner-only.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/shares/groups/{}", group.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert!(
        share_groups::Entity::find_by_id(group.id)
            .one(&context.db)
            .await
            .unwrap()
            .is_some(),
        "co-owner delete must not drop the group"
    );

    // The owner can delete it.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/shares/groups/{}", group.id))
        .cookie(alice.jwt.clone())
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::NO_CONTENT,
        "owner may delete the group"
    );
}

/// Share `folder` with `recipient` at `role`, signing the post-share
/// roster of (owner co-owner, recipient at `role`).
macro_rules! share_folder_to {
    ($app:expr, $owner:expr, $recipient:expr, $role:expr, $folder:expr) => {{
        let members_after = vec![
            FolderListMemberSpec {
                user: &$owner,
                share_role: ShareRoleEnum::CoOwner,
                is_owner: true,
                signed_by: &$owner,
            },
            FolderListMemberSpec {
                user: &$recipient,
                share_role: $role,
                is_owner: false,
                signed_by: &$owner,
            },
        ];
        let envelope = build_folder_share_envelope(
            &$owner,
            &$recipient,
            $role,
            $folder.id,
            $owner.user_id,
            random_nonce(),
            now_secs(),
            &members_after,
            &$owner,
        );
        assert_eq!(
            post_share!($app, $owner, envelope).status(),
            StatusCode::CREATED,
            "folder share setup failed"
        );
    }};
}

/// Upload `new_file_id` into a shared folder as `uploader` (who becomes the
/// file owner), wrapping a key for every current member of the folder.
fn upload_into_folder_body(
    uploader: &TestUser,
    new_file_id: entity::Uuid,
    folder_id: entity::Uuid,
    members: &[(&TestUser, bool)],
    timestamp: i64,
) -> Value {
    let member_keys: Vec<(entity::Uuid, &str, bool)> = members
        .iter()
        .map(|(u, is_owner)| (u.user_id, "wrap", *is_owner))
        .collect();
    let event_signature = sign_no_recipient_event(
        uploader,
        new_file_id,
        AuditEventActionEnum::SharedFolderUpload,
        timestamp,
    );
    build_upload_multikey_body(
        new_file_id,
        folder_id,
        "child-hash",
        member_keys,
        timestamp,
        None,
        event_signature,
        timestamp,
    )
}

#[actix_web::test]
async fn test_revoked_owner_can_move_their_own_file_out_of_folder() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // Alice owns folder F and shares it with Bob (editor). Bob uploads his
    // own file X into F, then Alice revokes Bob from F. Bob no longer holds
    // a membership row on F, yet he owns X — he must still be able to pull
    // X back out to his own root.
    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "shared-folder");
    share_folder_to!(app, alice, bob, ShareRoleEnum::Editor, folder);

    let child_id = entity::Uuid::new_v4();
    let upload_ts = now_secs();
    let upload = upload_into_folder_body(
        &bob,
        child_id,
        folder.id,
        &[(&alice, false), (&bob, true)],
        upload_ts,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&upload)
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::CREATED,
        "bob uploads his own file into the shared folder"
    );

    // Alice revokes Bob from the folder.
    let revoke_ts = now_secs();
    let revoke_body = build_folder_revoke_body(
        &alice,
        &bob,
        folder.id,
        alice.user_id,
        ShareRoleEnum::Editor,
        revoke_ts,
        &[FolderListMemberSpec {
            user: &alice,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: &alice,
        }],
        &alice,
    );
    let resp = delete_share!(app, alice, folder.id, bob.user_id, revoke_body);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "revoke failed");

    // Bob has no membership row on the parent folder anymore.
    let bob_on_folder = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(bob_on_folder.is_none(), "bob is no longer a folder member");

    // Bob — still the owner of X — moves it out to his root.
    let move_ts = now_secs();
    let move_sig = sign_no_recipient_event(
        &bob,
        child_id,
        AuditEventActionEnum::SharedFolderMoveOut,
        move_ts,
    );
    let body = build_move_out_of_shared_body(child_id, None, move_sig, move_ts);
    let req = test::TestRequest::post()
        .uri("/api/storage/move-out-of-shared")
        .cookie(bob.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let resp_body = test::read_body(resp).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "revoked owner must still move their own file out: {}",
        String::from_utf8_lossy(&resp_body)
    );

    let moved = files::Entity::find_by_id(child_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.file_id, None, "X is back at bob's root");
    let bob_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(child_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob keeps his owner row on X");
    assert!(bob_row.is_owner);
    let _ = context;
}

#[actix_web::test]
async fn test_revoking_co_owner_relocates_their_owned_files_to_root() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // Alice owns folder F and shares it with Bob as co-owner. Bob uploads
    // his own file X into F. When Alice revokes Bob, X must not be orphaned
    // under a folder Bob can no longer reach: it relocates to Bob's root
    // (parent NULL) with his owner row intact, a system audit row records
    // it, and Alice — the revoker — can no longer read it.
    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "shared-folder");
    share_folder_to!(app, alice, bob, ShareRoleEnum::CoOwner, folder);

    let child_id = entity::Uuid::new_v4();
    let upload_ts = now_secs();
    let upload = upload_into_folder_body(
        &bob,
        child_id,
        folder.id,
        &[(&alice, false), (&bob, true)],
        upload_ts,
    );
    let req = test::TestRequest::post()
        .uri("/api/storage/upload-multikey")
        .cookie(bob.jwt.clone())
        .set_json(&upload)
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::CREATED,
        "bob uploads his own file into the shared folder"
    );

    // Alice holds a member row on X before the revoke (she was a folder
    // member when bob uploaded).
    assert!(
        user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(child_id))
            .filter(user_files::Column::UserId.eq(alice.user_id))
            .one(&context.db)
            .await
            .unwrap()
            .is_some(),
        "alice has a row on X before the revoke"
    );

    // Alice revokes Bob's co-owner access to the folder.
    let revoke_ts = now_secs();
    let revoke_body = build_folder_revoke_body(
        &alice,
        &bob,
        folder.id,
        alice.user_id,
        ShareRoleEnum::CoOwner,
        revoke_ts,
        &[FolderListMemberSpec {
            user: &alice,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: &alice,
        }],
        &alice,
    );
    let resp = delete_share!(app, alice, folder.id, bob.user_id, revoke_body);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "revoke failed");

    // X relocated to bob's root with his owner row intact.
    let moved = files::Entity::find_by_id(child_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(moved.file_id, None, "X relocated to bob's root");
    let bob_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(child_id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob keeps his owner row on X");
    assert!(bob_row.is_owner);

    // The revoker can no longer read X — her member row is gone.
    let alice_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(child_id))
        .filter(user_files::Column::UserId.eq(alice.user_id))
        .one(&context.db)
        .await
        .unwrap();
    assert!(
        alice_row.is_none(),
        "the revoker must lose access to the relocated file"
    );

    // A system-attributed relocation audit row exists for X.
    let relocate_rows = entity::share_events::Entity::find()
        .filter(entity::share_events::Column::Action.eq("relocated_on_revoke"))
        .filter(entity::share_events::Column::FileId.eq(child_id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(relocate_rows.len(), 1, "one relocation audit row for X");
    assert!(
        relocate_rows[0].sender_id.is_none(),
        "relocation is system-attributed (no signer)"
    );
    let _ = context;
}
