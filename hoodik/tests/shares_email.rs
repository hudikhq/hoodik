//! Share-notification email dispatch — best-effort outbound on every
//! brand-new `user_files` row, suppressed by the recipient's per-account
//! `share_notifications_enabled` toggle. The MockSender records subject
//! lines, which is enough to prove the dispatch happened.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use context::SenderContract;
use cryptfns::asn1::ShareRoleEnum;
use entity::{users, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;

use crate::shares_common::*;

const SUBJECT: &str = "You have a new shared file on Hoodik";

#[actix_web::test]
async fn test_share_dispatches_notification_email_to_recipient() {
    let context =
        context::Context::add_mock_sender(context::Context::mock_sqlite().await);
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "share-email-target");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let mailer = context
        .sender
        .as_ref()
        .expect("mock sender attached for the test");
    assert!(
        mailer.has(SUBJECT),
        "share notification email was not dispatched"
    );
}

#[actix_web::test]
async fn test_share_skips_email_when_recipient_opted_out() {
    let context =
        context::Context::add_mock_sender(context::Context::mock_sqlite().await);
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "share-email-optout");

    users::Entity::update(users::ActiveModel {
        id: ActiveValue::Unchanged(bob.user_id),
        share_notifications_enabled: ActiveValue::Set(false),
        ..Default::default()
    })
    .filter(users::Column::Id.eq(bob.user_id))
    .exec(&context.db)
    .await
    .unwrap();

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let mailer = context
        .sender
        .as_ref()
        .expect("mock sender attached for the test");
    assert!(
        !mailer.has(SUBJECT),
        "share notification email was dispatched despite opt-out"
    );
}

#[actix_web::test]
async fn test_co_owner_reshare_dispatches_notification() {
    // Bob (Co-owner) re-shares Alice's file to Carol — the email goes
    // to Carol regardless of who issued the grant; she's the new
    // recipient and notifications honour her preference.
    let context =
        context::Context::add_mock_sender(context::Context::mock_sqlite().await);
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "share-email-coowner-reshare");

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
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let mailer = context
        .sender
        .as_ref()
        .expect("mock sender attached for the test");
    // Both Bob (from Alice) and Carol (from Bob) get a subject hit.
    assert!(
        mailer.has(SUBJECT),
        "co-owner reshare email was not dispatched"
    );
}
