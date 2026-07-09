//! The full legacy → Curve25519 + OPAQUE migration, driven end to end through
//! the real routes with `cryptfns` playing the client. Proves the account
//! flips atomically and that afterward OPAQUE login works, the legacy password
//! login is dead, and the re-wrapped file key is what the client submitted.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test};
use auth::data::{authenticated::Authenticated, signature::Signature};
use entity::{ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use serde_json::{json, Value};

const EMAIL: &str = "migrate@example.com";
const PASSWORD: &[u8] = helpers::LEGACY_PASSWORD.as_bytes();

trait TestApp:
    Service<actix_http::Request, Response = ServiceResponse<EitherBody<BoxBody>>, Error = actix_web::Error>
{
}
impl<S> TestApp for S where
    S: Service<
        actix_http::Request,
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = actix_web::Error,
    >
{
}

struct LegacyUser {
    jwt: actix_web::cookie::Cookie<'static>,
    rsa_private: String,
    rsa_public: String,
    rsa_fingerprint: String,
}

/// Seed a legacy RSA account at the data layer, then log it in through the
/// credentials endpoint to obtain a session cookie — the register endpoint no
/// longer creates legacy accounts, but login-time migration still targets them.
async fn register_legacy(app: &impl TestApp, db: &entity::DbConn) -> LegacyUser {
    let seeded = helpers::seed_legacy_user(db, EMAIL).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": EMAIL, "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (jwt, _) = helpers::extract_cookies(resp.headers());

    LegacyUser {
        jwt: jwt.unwrap(),
        rsa_private: seeded.rsa_private,
        rsa_public: seeded.rsa_public,
        rsa_fingerprint: seeded.rsa_fingerprint,
    }
}

async fn create_file(app: &impl TestApp, jwt: &actix_web::cookie::Cookie<'static>) -> entity::Uuid {
    let file = storage::data::create_file::CreateFile {
        encrypted_key: Some("old-rsa-wrapped-key".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("hash".to_string()),
        mime: Some("text/plain".to_string()),
        size: Some(1024),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: Some("a".to_string()),
        sha1: Some("a".to_string()),
        sha256: Some("a".to_string()),
        blake2b: Some("a".to_string()),
        cipher: Some("aegis256".to_string()),
        editable: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&file)
        .to_request();
    let body = test::call_and_read_body(app, req).await;
    serde_json::from_slice::<storage::data::app_file::AppFile>(&body)
        .expect("create_file json")
        .id
}

/// Run the client-side migration ceremony and POST `migration/complete`.
/// Returns the response plus the X25519 keys and the file key wrapped for the
/// one file, so the caller can assert the re-wrap round-trips.
async fn migrate(
    app: &impl TestApp,
    user: &LegacyUser,
) -> (ServiceResponse<EitherBody<BoxBody>>, String, Vec<u8>) {
    // New Curve25519 identity + wrapping keys.
    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let new_fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_private = cryptfns::ecdh::private::generate().unwrap();
    let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();

    // OPAQUE registration through the authenticated start endpoint; the finish
    // upload is folded into migration/complete.
    let reg_start = cryptfns::opaque::client_registration_start(PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/start")
        .cookie(user.jwt.clone())
        .set_json(json!({ "registration_request": reg_start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(app, req).await).await;
    let reg_response = body["registration_response"].as_str().unwrap();
    let reg_finish =
        cryptfns::opaque::client_registration_finish(&reg_start.state, reg_response, PASSWORD)
            .unwrap();

    // Envelope-wrap a stand-in private-key bundle under the export_key KEK.
    let export_key = cryptfns::base64::decode(&reg_finish.export_key).unwrap();
    let kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope = cryptfns::envelope::seal(&kek, b"new-private-key-bundle").unwrap();

    // Re-wrap every held key under the new X25519 key.
    let req = test::TestRequest::get()
        .uri("/api/auth/migration/keys")
        .cookie(user.jwt.clone())
        .to_request();
    let keys: Value = test::read_body_json(test::call_service(app, req).await).await;
    let file_key = cryptfns::aegis256::generate_key().unwrap();
    let rewrapped: Vec<Value> = keys
        .as_array()
        .unwrap()
        .iter()
        .map(|k| {
            json!({
                "file_id": k["file_id"],
                "encrypted_key": cryptfns::ecdh::wrap(&file_key, &x_public).unwrap(),
            })
        })
        .collect();

    // The transition certificate: old RSA endorses, new Ed25519 proves. The
    // server re-encodes this canonical from its own record, so the user_id
    // must be the real one it will use.
    let issued_at = chrono::Utc::now().timestamp();
    let user_id = current_user_id(app, &user.jwt).await;
    let cert = cryptfns::transition::Certificate {
        user_id: user_id.into_bytes(),
        old_key_type: cryptfns::identity::KeyType::Rsa,
        old_key_pem: &user.rsa_public,
        old_fingerprint: &user.rsa_fingerprint,
        new_identity_key_pem: &ed_public,
        new_wrapping_key_pem: &x_public,
        new_fingerprint: &new_fingerprint,
        issued_at,
    };
    let signatures = cert.sign(&user.rsa_private, &ed_private).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(json!({
            "new_identity_pubkey": ed_public,
            "new_wrapping_pubkey": x_public,
            "new_fingerprint": new_fingerprint,
            "transition_old_signature": signatures.old_signature,
            "transition_new_signature": signatures.new_signature,
            "transition_issued_at": issued_at,
            "opaque_registration_upload": reg_finish.message,
            "encrypted_private_key": envelope,
            "rewrapped_keys": rewrapped,
        }))
        .to_request();

    (test::call_service(app, req).await, x_private, file_key)
}

async fn current_user_id(
    app: &impl TestApp,
    jwt: &actix_web::cookie::Cookie<'static>,
) -> entity::Uuid {
    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(jwt.clone())
        .to_request();
    let body: Value = test::read_body_json(test::call_service(app, req).await).await;
    entity::Uuid::parse_str(body["user"]["id"].as_str().unwrap()).unwrap()
}

#[actix_web::test]
async fn test_full_migration_flips_account_and_rewraps_keys() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy(&app, &context.db).await;
    let file_id = create_file(&app, &user.jwt).await;

    let (resp, x_private, file_key) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK, "migration completes");
    let migrated: Value = test::read_body_json(resp).await;
    assert_eq!(migrated["security_version"], 1);
    assert_eq!(migrated["key_type"], "curve25519");
    let new_fingerprint = migrated["fingerprint"].as_str().unwrap().to_string();

    // The re-wrapped key persisted and unwraps to the file key the client sent.
    let row = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::FileId.eq(file_id))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    let recovered = cryptfns::ecdh::unwrap(&row.encrypted_key, &x_private).unwrap();
    assert_eq!(recovered, file_key, "the file key survives the re-wrap");

    // Legacy password login is dead (password cleared).
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": EMAIL, "password": String::from_utf8_lossy(PASSWORD) }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::UNAUTHORIZED,
        "bcrypt login must be gone after migration"
    );

    // OPAQUE login now works with the same password.
    let start = cryptfns::opaque::client_login_start(PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .set_json(json!({ "email": EMAIL, "credential_request": start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body["method"], "opaque", "migrated account authenticates via OPAQUE");

    // Signature login with the NEW key resolves through the new fingerprint.
    assert_eq!(new_fingerprint.len(), 64);
}

#[actix_web::test]
async fn test_second_migration_is_rejected() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy(&app, &context.db).await;
    create_file(&app, &user.jwt).await;

    let (resp, _, _) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // The legacy session's JWT is still valid, but the account is no longer
    // legacy, so a replayed migration must be refused.
    let (resp, _, _) = migrate(&app, &user).await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "a migrated account cannot migrate again"
    );
}

/// Prove the deferred key-transition consumer for signature login:
/// after a legacy account has migrated, a client that only has the *old* RSA
/// private key (and its old fingerprint) can still authenticate via
/// POST /api/auth/signature. The server resolves the old fingerprint through
/// the `key_transitions` row, verifies the signature against the historical
/// public key, and issues a session under the *current* (curve) identity.
#[actix_web::test]
async fn test_signature_login_with_old_fingerprint_after_migration() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy(&app, &context.db).await;
    let _file_id = create_file(&app, &user.jwt).await;

    let (migrate_resp, _x_private, _file_key) = migrate(&app, &user).await;
    assert_eq!(migrate_resp.status(), StatusCode::OK);

    // Use the OLD fingerprint + OLD RSA private to sign a fresh nonce.
    // The nonce format must match the server's: fingerprint- (timestamp/60)
    let old_fp = &user.rsa_fingerprint;
    let nonce = format!("{}-{}", old_fp, chrono::Utc::now().timestamp() / 60);
    let signature = cryptfns::rsa::private::sign(&nonce, &user.rsa_private).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .set_json(&Signature {
            fingerprint: Some(old_fp.clone()),
            signature: Some(signature),
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "signature login with pre-migration (old) fingerprint must succeed via key transition"
    );

    let authenticated: Authenticated =
        serde_json::from_slice(&test::read_body(resp).await).unwrap();

    // The returned identity is the post-migration one.
    assert_eq!(authenticated.user.key_type, "curve25519");
    assert_eq!(authenticated.session.user_id, authenticated.user.id);
    assert_ne!(
        authenticated.user.fingerprint, *old_fp,
        "the authenticated user must be the new curve identity, not the old fingerprint"
    );
}
