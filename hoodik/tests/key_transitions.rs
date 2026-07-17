//! Storage-level checks for the key-transition foundation: the migration
//! applies, a real certificate persists, and `old_fingerprint` is unique so a
//! key can be superseded at most once.

use chrono::Utc;
use cryptfns::identity::KeyType;
use cryptfns::transition::Certificate;
use entity::{key_transitions, users, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, Uuid};

fn generate_legacy_rsa() -> (String, String, String) {
    let private = cryptfns::rsa::private::generate().unwrap();
    let private_pem = cryptfns::rsa::private::to_string(&private).unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_pem = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();
    (private_pem, public_pem, fingerprint)
}

fn generate_curve25519() -> (String, String, String, String) {
    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let ed_fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_private = cryptfns::ecdh::private::generate().unwrap();
    let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();
    (ed_private, ed_public, ed_fingerprint, x_public)
}

#[actix_web::test]
async fn test_key_transition_certificate_persists() {
    let context = context::Context::mock_sqlite().await;

    let (rsa_private, rsa_public, rsa_fingerprint) = generate_legacy_rsa();
    let (ed_private, ed_public, ed_fingerprint, x_public) = generate_curve25519();

    let user_id = Uuid::new_v4();
    users::Entity::insert(users::ActiveModel {
        id: ActiveValue::Set(user_id),
        role: ActiveValue::NotSet,
        quota: ActiveValue::NotSet,
        email: ActiveValue::Set("legacy@example.com".to_string()),
        password: ActiveValue::Set(Some("bcrypt".to_string())),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set(rsa_public.clone()),
        fingerprint: ActiveValue::Set(rsa_fingerprint.clone()),
        key_type: ActiveValue::Set("rsa".to_string()),
        wrapping_pubkey: ActiveValue::NotSet,
        security_version: ActiveValue::Set(0),
        opaque_password_file: ActiveValue::NotSet,
        encrypted_private_key: ActiveValue::NotSet,
        email_verified_at: ActiveValue::Set(Some(Utc::now().timestamp())),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
        updated_at: ActiveValue::Set(Utc::now().timestamp()),
        share_notifications_enabled: ActiveValue::Set(true),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();

    let issued_at = Utc::now().timestamp();
    let cert = Certificate {
        user_id: user_id.into_bytes(),
        old_key_type: KeyType::Rsa,
        old_key_pem: &rsa_public,
        old_fingerprint: &rsa_fingerprint,
        new_identity_key_pem: &ed_public,
        new_wrapping_key_pem: &x_public,
        new_fingerprint: &ed_fingerprint,
        issued_at,
    };
    let signatures = cert.sign(&rsa_private, &ed_private).unwrap();

    // The server re-encodes the canonical from its own record and verifies
    // before persisting — never trusting wire-supplied certificate bytes.
    cert.verify(&signatures).unwrap();

    key_transitions::Entity::insert(key_transitions::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_id: ActiveValue::Set(user_id),
        old_fingerprint: ActiveValue::Set(rsa_fingerprint.clone()),
        old_key_spki: ActiveValue::Set(KeyType::Rsa.member_pubkey_der(&rsa_public).unwrap()),
        old_key_type: ActiveValue::Set("rsa".to_string()),
        new_fingerprint: ActiveValue::Set(ed_fingerprint.clone()),
        new_identity_key_pem: ActiveValue::Set(ed_public.clone()),
        new_wrapping_key_pem: ActiveValue::Set(x_public.clone()),
        old_signature: ActiveValue::Set(cryptfns::base64::decode(&signatures.old_signature).unwrap()),
        new_signature: ActiveValue::Set(cryptfns::base64::decode(&signatures.new_signature).unwrap()),
        issued_at: ActiveValue::Set(issued_at),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();

    let row = key_transitions::Entity::find()
        .filter(key_transitions::Column::OldFingerprint.eq(rsa_fingerprint.clone()))
        .one(&context.db)
        .await
        .unwrap()
        .expect("transition row persists and is retrievable by old fingerprint");
    assert_eq!(row.new_fingerprint, ed_fingerprint);
    assert_eq!(row.user_id, user_id);
    assert_eq!(row.new_identity_key_pem, ed_public);
    assert_eq!(row.new_wrapping_key_pem, x_public);

    // A certificate rebuilt from the row's stored components alone — no
    // dependence on the account's live keys — verifies under both signatures.
    // This is the property that keeps an intermediate hop verifiable after the
    // account has rotated past it.
    let old_pem = KeyType::Rsa.pem_from_member_der(&row.old_key_spki).unwrap();
    let rebuilt = Certificate {
        user_id: user_id.into_bytes(),
        old_key_type: KeyType::Rsa,
        old_key_pem: &old_pem,
        old_fingerprint: &row.old_fingerprint,
        new_identity_key_pem: &row.new_identity_key_pem,
        new_wrapping_key_pem: &row.new_wrapping_key_pem,
        new_fingerprint: &row.new_fingerprint,
        issued_at: row.issued_at,
    };
    rebuilt
        .verify(&cryptfns::transition::Signatures {
            old_signature: cryptfns::base64::encode(&row.old_signature),
            new_signature: cryptfns::base64::encode(&row.new_signature),
        })
        .expect("the persisted row self-verifies from its stored components");
}

#[actix_web::test]
async fn test_old_fingerprint_is_unique() {
    let context = context::Context::mock_sqlite().await;

    let (_, _, rsa_fingerprint) = generate_legacy_rsa();
    let user_id = Uuid::new_v4();
    users::Entity::insert(users::ActiveModel {
        id: ActiveValue::Set(user_id),
        role: ActiveValue::NotSet,
        quota: ActiveValue::NotSet,
        email: ActiveValue::Set("dup@example.com".to_string()),
        password: ActiveValue::Set(Some("bcrypt".to_string())),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set("pubkey".to_string()),
        fingerprint: ActiveValue::Set(rsa_fingerprint.clone()),
        key_type: ActiveValue::Set("rsa".to_string()),
        wrapping_pubkey: ActiveValue::NotSet,
        security_version: ActiveValue::Set(0),
        opaque_password_file: ActiveValue::NotSet,
        encrypted_private_key: ActiveValue::NotSet,
        email_verified_at: ActiveValue::Set(Some(Utc::now().timestamp())),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
        updated_at: ActiveValue::Set(Utc::now().timestamp()),
        share_notifications_enabled: ActiveValue::Set(true),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();

    let row = |fp: &str| key_transitions::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_id: ActiveValue::Set(user_id),
        old_fingerprint: ActiveValue::Set(fp.to_string()),
        old_key_spki: ActiveValue::Set(vec![1, 2, 3]),
        old_key_type: ActiveValue::Set("rsa".to_string()),
        new_fingerprint: ActiveValue::Set("new".to_string()),
        new_identity_key_pem: ActiveValue::Set("new-identity-pem".to_string()),
        new_wrapping_key_pem: ActiveValue::Set("new-wrapping-pem".to_string()),
        old_signature: ActiveValue::Set(vec![4, 5]),
        new_signature: ActiveValue::Set(vec![6, 7]),
        issued_at: ActiveValue::Set(0),
        created_at: ActiveValue::Set(0),
    };

    key_transitions::Entity::insert(row(&rsa_fingerprint))
        .exec_without_returning(&context.db)
        .await
        .unwrap();

    let second = key_transitions::Entity::insert(row(&rsa_fingerprint))
        .exec_without_returning(&context.db)
        .await;
    assert!(second.is_err(), "a key may be superseded at most once");
}
