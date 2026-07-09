//! RSA and Curve25519 accounts side by side: registration, Ed25519
//! signature login, and the four sender/recipient share combinations.
//!
//! Curve25519 accounts sign with their Ed25519 identity key and receive
//! file keys as X25519 ECDH wraps; RSA accounts keep the legacy behaviour.
//! Every test drives the real HTTP routes end-to-end.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use auth::data::{authenticated::Authenticated, signature::Signature};
use cryptfns::asn1::ShareRoleEnum;
use entity::{user_files, users, ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;

use crate::shares_common::*;

async fn share_row(
    db: &entity::DbConn,
    file_id: entity::Uuid,
    user_id: entity::Uuid,
) -> user_files::Model {
    user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file_id))
        .filter(user_files::Column::UserId.eq(user_id))
        .one(db)
        .await
        .unwrap()
        .expect("user_files row should exist")
}

#[actix_web::test]
async fn test_register_curve25519_user_and_signature_login() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_curve25519_user!(app, carol, "carol@example.com");

    let row = users::Entity::find_by_id(carol.user_id)
        .one(&context.db)
        .await
        .unwrap()
        .expect("registered user row");
    assert_eq!(row.key_type, "curve25519");
    assert_eq!(row.wrapping_pubkey, carol.wrapping_public_pem);

    let nonce = auth::mock::generate_fingerprint_nonce(&carol.fingerprint);
    let signature = cryptfns::ed25519::private::sign(&nonce, &carol.private_pem).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .set_json(&Signature {
            fingerprint: Some(carol.fingerprint.clone()),
            signature: Some(signature),
        })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (jwt, _) = extract_cookies(resp.headers());
    assert!(jwt.is_some(), "signature login must set a session cookie");

    let authenticated: Authenticated =
        serde_json::from_slice(&test::read_body(resp).await).unwrap();
    assert_eq!(authenticated.user.id, carol.user_id);
    assert_eq!(authenticated.session.user_id, carol.user_id);
}

#[actix_web::test]
async fn test_registration_rejects_bad_curve25519_input() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    macro_rules! register_expecting_422 {
        ($app:expr, $payload:expr) => {{
            let req = test::TestRequest::post()
                .uri("/api/auth/register")
                .set_json(&$payload)
                .to_request();
            let resp = test::call_service(&$app, req).await;
            assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        }};
    }

    let (_, public_pem, fingerprint, _, wrapping_public_pem) = generate_curve25519_keypair();

    // The fingerprint must equal the SPKI derivation of the submitted pubkey.
    let (_, _, other_fingerprint, _, _) = generate_curve25519_keypair();
    register_expecting_422!(
        app,
        make_create_curve25519_user(
            "wrong-fingerprint@example.com",
            &public_pem,
            &other_fingerprint,
            &wrapping_public_pem,
        )
    );

    // A curve25519 account without an X25519 wrapping key can never
    // receive a file key.
    let mut missing_wrapping = make_create_curve25519_user(
        "missing-wrapping@example.com",
        &public_pem,
        &fingerprint,
        &wrapping_public_pem,
    );
    missing_wrapping.wrapping_pubkey = None;
    register_expecting_422!(app, missing_wrapping);

    // The RSA key wraps and signs; a second key is a client bug.
    let (_, rsa_public_pem, rsa_fingerprint) = generate_keypair();
    let mut rsa_with_wrapping =
        make_create_user("rsa-with-wrapping@example.com", &rsa_public_pem, &rsa_fingerprint);
    rsa_with_wrapping.wrapping_pubkey = Some(wrapping_public_pem.clone());
    register_expecting_422!(app, rsa_with_wrapping);

    // Unknown key types are rejected outright.
    let mut unknown_key_type = make_create_curve25519_user(
        "ed448@example.com",
        &public_pem,
        &fingerprint,
        &wrapping_public_pem,
    );
    unknown_key_type.key_type = Some("ed448".to_string());
    register_expecting_422!(app, unknown_key_type);

    let _ = context;
}

#[actix_web::test]
async fn test_share_matrix_all_four_combos() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_user!(app, bob, "bob@example.com");
    register_curve25519_user!(app, carol, "carol@example.com");
    register_curve25519_user!(app, dave, "dave@example.com");

    let file_key: &[u8] = b"0123456789abcdef0123456789abcdef";

    // rsa -> rsa: the pre-migration path must keep working unchanged.
    let alice_file = create_file!(app, alice, "matrix-rsa-owned");
    let bob_wrap = cryptfns::rsa::public::encrypt("deadbeef", &bob.public_pem).unwrap();
    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        alice_file.id,
        vec![(alice_file.id, cryptfns::base64::decode(&bob_wrap).unwrap())],
        random_nonce(),
        now_secs(),
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);
    let row = share_row(&context.db, alice_file.id, bob.user_id).await;
    assert_eq!(row.encrypted_key, bob_wrap);
    assert_eq!(
        cryptfns::rsa::private::decrypt(&row.encrypted_key, &bob.private_pem).unwrap(),
        "deadbeef"
    );

    // rsa -> curve25519: RSA owner wraps the file key for an X25519 recipient.
    let carol_wrap =
        cryptfns::ecdh::wrap(file_key, carol.wrapping_public_pem.as_ref().unwrap()).unwrap();
    let envelope = build_share_envelope(
        &alice,
        &carol,
        ShareRoleEnum::Reader,
        alice_file.id,
        vec![(alice_file.id, cryptfns::base64::decode(&carol_wrap).unwrap())],
        random_nonce(),
        now_secs(),
    );
    assert_eq!(post_share!(app, alice, envelope).status(), StatusCode::CREATED);
    let row = share_row(&context.db, alice_file.id, carol.user_id).await;
    assert_eq!(row.encrypted_key, carol_wrap);
    assert_eq!(
        cryptfns::ecdh::unwrap(&row.encrypted_key, carol.wrapping_private_pem.as_ref().unwrap())
            .unwrap()
            .as_slice(),
        file_key
    );

    // The curve25519 owner's own key wrap must round-trip too.
    let carol_file = create_file!(app, carol, "matrix-curve25519-owned");
    let owner_row = share_row(&context.db, carol_file.id, carol.user_id).await;
    assert_eq!(
        cryptfns::ecdh::unwrap(
            &owner_row.encrypted_key,
            carol.wrapping_private_pem.as_ref().unwrap(),
        )
        .unwrap(),
        b"deadbeef".to_vec()
    );

    // curve25519 -> rsa: Ed25519-signed envelope, classic RSA wrap.
    let bob_wrap_from_carol = cryptfns::rsa::public::encrypt("deadbeef", &bob.public_pem).unwrap();
    let envelope = build_share_envelope(
        &carol,
        &bob,
        ShareRoleEnum::Reader,
        carol_file.id,
        vec![(
            carol_file.id,
            cryptfns::base64::decode(&bob_wrap_from_carol).unwrap(),
        )],
        random_nonce(),
        now_secs(),
    );
    assert_eq!(post_share!(app, carol, envelope).status(), StatusCode::CREATED);
    let row = share_row(&context.db, carol_file.id, bob.user_id).await;
    assert_eq!(row.encrypted_key, bob_wrap_from_carol);
    assert_eq!(
        cryptfns::rsa::private::decrypt(&row.encrypted_key, &bob.private_pem).unwrap(),
        "deadbeef"
    );

    // curve25519 -> curve25519: Ed25519-signed envelope, X25519 wrap.
    let dave_wrap =
        cryptfns::ecdh::wrap(file_key, dave.wrapping_public_pem.as_ref().unwrap()).unwrap();
    let envelope = build_share_envelope(
        &carol,
        &dave,
        ShareRoleEnum::Reader,
        carol_file.id,
        vec![(carol_file.id, cryptfns::base64::decode(&dave_wrap).unwrap())],
        random_nonce(),
        now_secs(),
    );
    assert_eq!(post_share!(app, carol, envelope).status(), StatusCode::CREATED);
    let row = share_row(&context.db, carol_file.id, dave.user_id).await;
    assert_eq!(row.encrypted_key, dave_wrap);
    assert_eq!(
        cryptfns::ecdh::unwrap(&row.encrypted_key, dave.wrapping_private_pem.as_ref().unwrap())
            .unwrap()
            .as_slice(),
        file_key
    );
}

#[actix_web::test]
async fn test_curve25519_member_signature_verifies() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_curve25519_user!(app, carol, "carol@example.com");
    register_user!(app, bob, "bob@example.com");
    let file = create_file!(app, carol, "member-sig-ed25519");

    let timestamp = now_secs();
    let envelope = build_share_envelope(
        &carol,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped-for-bob".to_vec())],
        random_nonce(),
        timestamp,
    );
    let envelope =
        inject_member_signature(envelope, &carol, &bob, ShareRoleEnum::Reader, timestamp);
    let resp = post_share!(app, carol, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = share_row(&context.db, file.id, bob.user_id).await;
    assert!(
        row.member_signature.is_some(),
        "Ed25519-signed member signature must persist after verification"
    );
}

#[actix_web::test]
async fn test_member_signature_binds_curve25519_recipient() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, alice, "alice@example.com");
    register_curve25519_user!(app, dave, "dave@example.com");
    let file = create_file!(app, alice, "member-sig-x25519-recipient");

    let timestamp = now_secs();
    let wrapped = cryptfns::ecdh::wrap(
        b"file-key-for-dave",
        dave.wrapping_public_pem.as_ref().expect("dave wrapping key"),
    )
    .unwrap();
    let envelope = build_share_envelope(
        &alice,
        &dave,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, wrapped.into_bytes())],
        random_nonce(),
        timestamp,
    );
    let envelope =
        inject_member_signature(envelope, &alice, &dave, ShareRoleEnum::Reader, timestamp);
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let row = share_row(&context.db, file.id, dave.user_id).await;
    assert!(
        row.member_signature.is_some(),
        "member signature over an SPKI-DER curve25519 recipient must verify and persist"
    );
}
