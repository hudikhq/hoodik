//! Admin kill-switch tests. Verifies that flipping
//! `Settings.sharing.enabled` to `false` turns the capability response
//! into a hidden-feature response, makes every share route 503, and
//! is reversible without data loss.

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

#[actix_web::test]
async fn test_capability_reports_disabled_when_settings_flipped() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    context.settings.inner().await.sharing.set_enabled(false);

    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let caps: Capabilities = test::read_body_json(resp).await;
    assert!(!caps.sharing.enabled);
}

#[actix_web::test]
async fn test_endpoints_return_503_when_disabled() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let file = create_file!(app, alice, "kill-switch-target");

    context.settings.inner().await.sharing.set_enabled(false);

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
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "sharing_disabled");

    let revoke_body =
        build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke_body);
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/{}", file.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/mine/by/{}", alice.user_id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let req = test::TestRequest::get()
        .uri("/api/shares/events")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Capability endpoint itself remains reachable so clients can detect
    // the disabled state and hide their UI fail-closed.
    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_flip_back_restores_endpoints() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let file = create_file!(app, alice, "kill-switch-cycle");

    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    // Disable sharing — existing rows must not vanish.
    context.settings.inner().await.sharing.set_enabled(false);

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
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let preserved = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob's row must survive the disable toggle");
    assert_eq!(preserved.share_role, "editor");

    // Re-enable and confirm a new submission succeeds. Bob still holds
    // his editor row, so the caller's downgrade to reader exercises the
    // role_change path — the envelope's audit signature must cover the
    // role_change canonical, not the legacy `grant` canonical.
    context.settings.inner().await.sharing.set_enabled(true);

    let envelope = build_role_change_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-after-reenable".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let post_row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file.id))
        .filter(user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob's row exists after re-enable");
    assert_eq!(post_row.share_role, "reader");
}

#[actix_web::test]
async fn test_admin_settings_round_trip_flips_sharing_enabled() {
    // An admin can flip the kill switch from the SPA via
    // the existing `/api/admin/settings` PUT. The first registered user
    // is the admin (auth/contracts/register.rs:47-48). After PUT the
    // capability endpoint reflects the new state without a restart.
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");

    let req = test::TestRequest::get()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let mut payload: Value = test::read_body_json(resp).await;
    assert_eq!(payload["sharing"]["enabled"], true);

    payload["sharing"]["enabled"] = Value::Bool(false);
    let req = test::TestRequest::put()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let echoed: Value = test::read_body_json(resp).await;
    assert_eq!(echoed["sharing"]["enabled"], false);

    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let caps: Capabilities = test::read_body_json(resp).await;
    assert!(!caps.sharing.enabled);

    payload["sharing"]["enabled"] = Value::Bool(true);
    let req = test::TestRequest::put()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let req = test::TestRequest::get()
        .uri("/api/capabilities")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let caps: Capabilities = test::read_body_json(resp).await;
    assert!(caps.sharing.enabled);
}

#[actix_web::test]
async fn test_non_admin_cannot_flip_sharing_kill_switch() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");

    let req = test::TestRequest::get()
        .uri("/api/admin/settings")
        .cookie(alice.jwt.clone())
        .to_request();
    let payload: Value = test::read_body_json(test::call_service(&app, req).await).await;

    let mut tampered = payload.clone();
    tampered["sharing"]["enabled"] = Value::Bool(false);

    let req = test::TestRequest::put()
        .uri("/api/admin/settings")
        .cookie(bob.jwt.clone())
        .set_json(&tampered)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
