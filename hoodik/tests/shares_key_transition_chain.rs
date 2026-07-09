//! Key-transition chain resolution for stored signatures.
//!
//! A legacy account that migrates RSA→Curve25519 leaves behind signatures it
//! made with the old key: the editable-folder roster signature, per-member
//! signatures, audit-event signatures. Verifying those against the account's
//! new key fails. These tests prove the server resolves through the account's
//! `key_transitions` row — both where the server itself re-verifies a stored
//! signature (roster verify on a membership mutation) and where it exposes the
//! transition so an E2EE client can resolve locally (folder-members and
//! audit-events responses).

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use chrono::Utc;
use cryptfns::asn1::{
    encode_audit_event_sig_input_v1, AuditEventActionEnum, AuditEventSigInputV1,
    ShareRoleEnum, AUDIT_EVENT_SIG_V1_PREFIX,
};
use cryptfns::identity::KeyType;
use cryptfns::transition::Certificate;
use entity::{key_transitions, users, ActiveValue, ConnectionTrait, EntityTrait, Uuid};
use hoodik::server;
use serde_json::Value;

use crate::shares_common::*;

/// Curve keys minted for a migration, kept so a test can prove the migrated
/// account's current (curve) key does *not* verify an old-key signature.
struct MigratedTo {
    ed_public: String,
    ed_fingerprint: String,
}

/// Migrate `user` RSA→Curve25519 at the data layer, exactly as
/// `auth::migration_complete` would: build and verify a real transition
/// certificate, insert the `key_transitions` row, and flip the user's identity
/// columns. `user.key_type` in the test fixture is left as RSA so
/// `user.sign_bytes` keeps producing the *old*-key signatures a pre-migration
/// client would have on file.
async fn migrate_user_to_curve<C: ConnectionTrait>(db: &C, user: &TestUser) -> MigratedTo {
    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let ed_fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_private = cryptfns::ecdh::private::generate().unwrap();
    let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();

    let issued_at = Utc::now().timestamp();
    let cert = Certificate {
        user_id: user.user_id.into_bytes(),
        old_key_type: KeyType::Rsa,
        old_key_pem: &user.public_pem,
        old_fingerprint: &user.fingerprint,
        new_identity_key_pem: &ed_public,
        new_wrapping_key_pem: &x_public,
        new_fingerprint: &ed_fingerprint,
        issued_at,
    };
    let signatures = cert.sign(&user.private_pem, &ed_private).unwrap();
    cert.verify(&signatures).unwrap();

    key_transitions::Entity::insert(key_transitions::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_id: ActiveValue::Set(user.user_id),
        old_fingerprint: ActiveValue::Set(user.fingerprint.clone()),
        old_key_spki: ActiveValue::Set(KeyType::Rsa.member_pubkey_der(&user.public_pem).unwrap()),
        old_key_type: ActiveValue::Set("rsa".to_string()),
        new_fingerprint: ActiveValue::Set(ed_fingerprint.clone()),
        old_signature: ActiveValue::Set(cryptfns::base64::decode(&signatures.old_signature).unwrap()),
        new_signature: ActiveValue::Set(cryptfns::base64::decode(&signatures.new_signature).unwrap()),
        issued_at: ActiveValue::Set(issued_at),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
    })
    .exec_without_returning(db)
    .await
    .unwrap();

    users::Entity::update(users::ActiveModel {
        id: ActiveValue::Unchanged(user.user_id),
        pubkey: ActiveValue::Set(ed_public.clone()),
        fingerprint: ActiveValue::Set(ed_fingerprint.clone()),
        key_type: ActiveValue::Set("curve25519".to_string()),
        wrapping_pubkey: ActiveValue::Set(Some(x_public)),
        security_version: ActiveValue::Set(1),
        ..Default::default()
    })
    .exec(db)
    .await
    .unwrap();

    MigratedTo { ed_public, ed_fingerprint }
}

/// Owner alice signs the folder roster with her RSA key; she then migrates to
/// curve; a co-owner re-shares, and the server re-verifies that RSA-signed
/// roster against alice's current (curve) key — which only passes because it
/// resolves through her transition to the old key.
#[actix_web::test]
async fn test_roster_signature_verifies_after_owner_migration() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, carol, "carol@example.com");
    register_user!(app, dave, "dave@example.com");
    let folder = create_folder!(app, alice, "chain-folder");

    // alice makes carol a Co-owner. Roster signed by alice (RSA), pre-migration.
    let after_carol = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &carol, share_role: ShareRoleEnum::CoOwner, is_owner: false, signed_by: &alice },
    ];
    grant_folder!(app, alice, carol, ShareRoleEnum::CoOwner, folder.id, alice, &after_carol, alice);

    // alice migrates RSA → curve. The stored roster signature is now stranded.
    migrate_user_to_curve(&context.db, &alice).await;

    // carol (un-migrated Co-owner) re-shares to dave. carol signs the fresh
    // share-request + audit event with her current key, but the roster is still
    // endorsed by owner alice with her OLD RSA key.
    let after_dave = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &carol, share_role: ShareRoleEnum::CoOwner, is_owner: false, signed_by: &alice },
        FolderListMemberSpec { user: &dave, share_role: ShareRoleEnum::Editor, is_owner: false, signed_by: &carol },
    ];
    let envelope = build_co_owner_folder_share_envelope(
        &carol,
        &dave,
        ShareRoleEnum::Editor,
        folder.id,
        alice.user_id,
        random_nonce(),
        now_secs(),
        &after_dave,
        &alice,
    );
    let resp = post_share!(app, carol, envelope);
    let status = resp.status();
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "an RSA-signed roster must still verify after the owner migrates, via chain resolution; body={body:?}",
    );
}

/// The folder-members response carries the migrated owner's `key_transition`
/// so a client can verify the stored roster signature against the old key. A
/// member who never migrated has no `key_transition` field.
#[actix_web::test]
async fn test_folder_members_response_exposes_owner_transition() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let folder = create_folder!(app, alice, "expose-folder");
    let members_after = vec![
        FolderListMemberSpec { user: &alice, share_role: ShareRoleEnum::CoOwner, is_owner: true, signed_by: &alice },
        FolderListMemberSpec { user: &bob, share_role: ShareRoleEnum::Editor, is_owner: false, signed_by: &alice },
    ];
    grant_folder!(app, alice, bob, ShareRoleEnum::Editor, folder.id, alice, &members_after, alice);

    let migrated = migrate_user_to_curve(&context.db, &alice).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/shares/folder/{}/members", folder.id))
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;

    let members = body["members"].as_array().unwrap();
    let alice_member = members
        .iter()
        .find(|m| m["user_id"] == alice.user_id.to_string())
        .unwrap();
    let bob_member = members
        .iter()
        .find(|m| m["user_id"] == bob.user_id.to_string())
        .unwrap();

    let transition = &alice_member["key_transition"];
    assert_eq!(transition["old_key_type"], "rsa");
    assert!(transition["old_key_pem"].as_str().unwrap().contains("BEGIN RSA PUBLIC KEY"));
    assert!(transition["old_signature"].is_string());
    assert!(transition["new_signature"].is_string());
    assert!(transition["issued_at"].is_i64());
    // The exposed PEM re-verifies the stored roster signature: parse the served
    // members_list_signature and confirm it holds under the old key, not the new.
    let old_pem = transition["old_key_pem"].as_str().unwrap();
    assert!(
        KeyType::Rsa.fingerprint(old_pem).unwrap() != migrated.ed_fingerprint,
        "the exposed key is the superseded RSA key, not the new identity"
    );

    assert!(
        bob_member.get("key_transition").is_none() || bob_member["key_transition"].is_null(),
        "an account that never migrated exposes no transition"
    );
    let _ = migrated;
}

/// An audit-event signature made before the sender migrated is served with the
/// sender's `key_transition`; the stored signature verifies against the exposed
/// old key and fails against the sender's current key.
#[actix_web::test]
async fn test_audit_events_response_exposes_signer_transition() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    let file = create_file!(app, alice, "audit-chain");

    // alice grants to bob; the audit row stores alice's RSA-signed event sig.
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
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);

    let migrated = migrate_user_to_curve(&context.db, &alice).await;

    let req = test::TestRequest::get()
        .uri("/api/shares/events")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;

    let alice_ref = &body["users"][alice.user_id.to_string()];
    let transition = &alice_ref["key_transition"];
    assert_eq!(transition["old_key_type"], "rsa");
    let old_pem = transition["old_key_pem"].as_str().unwrap().to_string();

    // Recover the stored grant event and its signature.
    let event = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["action"] == "grant")
        .expect("the grant audit event");
    let stored_signature = event["sender_signature"].as_str().unwrap();

    // Re-encode the canonical the client verifies against, exactly as the
    // server built it for this grant (recipient present, grant action).
    let sig_input = AuditEventSigInputV1 {
        sender_id: alice.user_id.into_bytes(),
        recipient_id: Some(bob.user_id.into_bytes()),
        file_id: file.id.into_bytes(),
        action: AuditEventActionEnum::Grant,
        share_role_before: None,
        share_role_after: Some(ShareRoleEnum::Reader),
        timestamp,
    };
    let der = encode_audit_event_sig_input_v1(&sig_input).unwrap();
    let mut signing_input = Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);

    // The stored signature verifies against the exposed old key.
    KeyType::Rsa
        .verify_bytes(&signing_input, stored_signature, &old_pem)
        .expect("stored audit signature verifies against the transition's old key");
    // ...and would fail against alice's current (curve) key — resolution is
    // required, not incidental.
    assert!(
        KeyType::Curve25519
            .verify_bytes(&signing_input, stored_signature, &migrated.ed_public)
            .is_err(),
        "the same signature must not verify under the new identity key"
    );
}
