//! The full legacy → Curve25519 + OPAQUE migration, driven end to end through
//! the real routes with `cryptfns` playing the client. Proves the account
//! flips atomically and that afterward OPAQUE login works, the legacy password
//! login is dead, and the re-wrapped file key is what the client submitted.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test};
use auth::data::{authenticated::Authenticated, signature::Signature};
use entity::{
    migration_rewrap_staging, share_events, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait,
    QueryFilter,
};
use hoodik::server;
use serde_json::{json, Value};

const EMAIL: &str = "migrate@example.com";
const PASSWORD: &[u8] = helpers::LEGACY_PASSWORD.as_bytes();

trait TestApp:
    Service<actix_http::Request, Response = ServiceResponse<Self::Body>, Error = actix_web::Error>
{
    type Body: actix_web::body::MessageBody;
}
impl<S, B> TestApp for S
where
    B: actix_web::body::MessageBody,
    S: Service<
        actix_http::Request,
        Response = ServiceResponse<B>,
        Error = actix_web::Error,
    >,
{
    type Body = B;
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
    register_legacy_with(app, db, EMAIL).await
}

/// As [`register_legacy`], for a specific email. Tests that add to the suite
/// pass a distinct one, and the login carries a per-account source IP, so
/// neither dimension of the process-global login rate limiter (keyed on email
/// and on source IP) aliases across the parallel run.
async fn register_legacy_with(app: &impl TestApp, db: &entity::DbConn, email: &str) -> LegacyUser {
    let seeded = helpers::seed_legacy_user(db, email).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .insert_header(("cf-connecting-ip", email))
        .set_json(json!({ "email": email, "password": helpers::LEGACY_PASSWORD }))
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

/// Insert a public link owned by `user_id` over `file_id`, returning its id.
/// Migration only touches `encrypted_link_key`; the other columns are realistic
/// stand-ins.
async fn seed_link<C: entity::ConnectionTrait>(
    db: &C,
    user_id: entity::Uuid,
    file_id: entity::Uuid,
    encrypted_link_key: &str,
) -> entity::Uuid {
    use entity::{ActiveValue, EntityTrait};

    let link_id = entity::Uuid::new_v4();
    let now = chrono::Utc::now().timestamp();
    entity::links::Entity::insert(entity::links::ActiveModel {
        id: ActiveValue::Set(link_id),
        user_id: ActiveValue::Set(user_id),
        file_id: ActiveValue::Set(file_id),
        signature: ActiveValue::Set("link-signature".to_string()),
        downloads: ActiveValue::Set(0),
        encrypted_name: ActiveValue::Set("encrypted-name".to_string()),
        encrypted_link_key: ActiveValue::Set(encrypted_link_key.to_string()),
        encrypted_thumbnail: ActiveValue::Set(None),
        encrypted_file_key: ActiveValue::Set(None),
        created_at: ActiveValue::Set(now),
        expires_at: ActiveValue::Set(None),
    })
    .exec_without_returning(db)
    .await
    .unwrap();

    link_id
}

/// The pieces of a migration ceremony, split so a test can stage the re-wrapped
/// keys and complete separately (and tamper with either): the `complete` body
/// (which no longer carries the re-wraps), the re-wrapped file and link keys
/// staged through `migration/rewrap`, the X25519 private key, the file key used
/// for the re-wraps, and each link's `(id, re-wrapped value)`.
struct MigrationParts {
    complete_body: Value,
    rewrap_keys: Vec<Value>,
    rewrap_link_keys: Vec<Value>,
    x_private: String,
    file_key: Vec<u8>,
    link_rewraps: Vec<(entity::Uuid, String)>,
}

/// Build a full migration ceremony against `user`: fresh Curve25519 identity +
/// X25519 wrapping keys, the OPAQUE upload, and every held file key and owned
/// link key re-wrapped under the new wrapping key. The key set is fetched
/// through the paginated `migration/keys` cursor, exactly as the real client
/// walks it.
async fn build_migration_parts(app: &impl TestApp, user: &LegacyUser) -> MigrationParts {
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

    // Walk the paginated key cursor to its end, accumulating every file and link
    // key the account must re-wrap.
    let mut server_file_keys: Vec<Value> = Vec::new();
    let mut server_link_keys: Vec<Value> = Vec::new();
    let mut offset = 0i64;
    loop {
        let req = test::TestRequest::get()
            .uri(&format!("/api/auth/migration/keys?offset={offset}&limit=500"))
            .cookie(user.jwt.clone())
            .to_request();
        let page: Value = test::read_body_json(test::call_service(app, req).await).await;
        server_file_keys.extend(page["keys"].as_array().unwrap().iter().cloned());
        server_link_keys.extend(page["link_keys"].as_array().unwrap().iter().cloned());
        match page["next_offset"].as_i64() {
            Some(n) => offset = n,
            None => break,
        }
    }

    let file_key = cryptfns::aegis256::generate_key().unwrap();
    let rewrap_keys: Vec<Value> = server_file_keys
        .iter()
        .map(|k| {
            json!({
                "file_id": k["file_id"],
                "encrypted_key": cryptfns::ecdh::wrap(&file_key, &x_public).unwrap(),
            })
        })
        .collect();

    let mut link_rewraps = Vec::new();
    let rewrap_link_keys: Vec<Value> = server_link_keys
        .iter()
        .map(|k| {
            let link_id = entity::Uuid::parse_str(k["link_id"].as_str().unwrap()).unwrap();
            let file_id = k["file_id"].as_str().unwrap();
            let value = cryptfns::ecdh::wrap(&file_key, &x_public).unwrap();
            let signature = cryptfns::ed25519::private::sign(file_id, &ed_private).unwrap();
            link_rewraps.push((link_id, value.clone()));
            json!({ "link_id": link_id, "encrypted_link_key": value, "signature": signature })
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

    // The key-rotation audit event, signed by the new identity over the
    // canonical the server rebuilds from its own state.
    let audit_event_signature = cryptfns::transition::sign_key_rotation_audit(
        &user_id.into_bytes(),
        &user.rsa_fingerprint,
        &new_fingerprint,
        issued_at,
        &ed_private,
    )
    .unwrap();

    let complete_body = json!({
        "new_identity_pubkey": ed_public,
        "new_wrapping_pubkey": x_public,
        "new_fingerprint": new_fingerprint,
        "transition_old_signature": signatures.old_signature,
        "transition_new_signature": signatures.new_signature,
        "transition_issued_at": issued_at,
        "opaque_registration_upload": reg_finish.message,
        "encrypted_private_key": envelope,
        "audit_event_signature": audit_event_signature,
    });

    MigrationParts {
        complete_body,
        rewrap_keys,
        rewrap_link_keys,
        x_private,
        file_key,
        link_rewraps,
    }
}

/// Stage the re-wrapped keys through `migration/rewrap`, chunking the combined
/// file-then-link sequence to respect the server's per-request cap — exactly how
/// the real client batches. Asserts each batch is accepted.
async fn stage_rewraps(
    app: &impl TestApp,
    user: &LegacyUser,
    keys: &[Value],
    link_keys: &[Value],
) {
    const REWRAP_BATCH: usize = 500;
    let total = keys.len() + link_keys.len();
    let mut start = 0;
    while start < total {
        let end = (start + REWRAP_BATCH).min(total);
        let mut batch_keys = Vec::new();
        let mut batch_links = Vec::new();
        for i in start..end {
            if i < keys.len() {
                batch_keys.push(keys[i].clone());
            } else {
                batch_links.push(link_keys[i - keys.len()].clone());
            }
        }
        let req = test::TestRequest::post()
            .uri("/api/auth/migration/rewrap")
            .cookie(user.jwt.clone())
            .set_json(json!({ "keys": batch_keys, "link_keys": batch_links }))
            .to_request();
        let resp = test::call_service(app, req).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT, "rewrap batch staged");
        start = end;
    }
}

/// Run the whole ceremony: build it, stage the re-wraps in batches, then POST
/// `complete`. Returns the completion response alongside the X25519 private key,
/// the file key used for the re-wraps, and the per-link re-wrapped values.
async fn migrate(
    app: &impl TestApp,
    user: &LegacyUser,
) -> (
    ServiceResponse<impl actix_web::body::MessageBody>,
    String,
    Vec<u8>,
    Vec<(entity::Uuid, String)>,
) {
    let parts = build_migration_parts(app, user).await;
    stage_rewraps(app, user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    (
        test::call_service(app, req).await,
        parts.x_private,
        parts.file_key,
        parts.link_rewraps,
    )
}

/// Count the migration staging rows currently held for a user.
async fn staging_count<C: entity::ConnectionTrait>(db: &C, user_id: entity::Uuid) -> u64 {
    migration_rewrap_staging::Entity::find()
        .filter(migration_rewrap_staging::Column::UserId.eq(user_id))
        .count(db)
        .await
        .unwrap()
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

    let (resp, x_private, file_key, _link_rewraps) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK, "migration completes");
    let migrated: Value = test::read_body_json(resp).await;
    assert_eq!(migrated["security_version"], 1);
    assert_eq!(migrated["key_type"], "curve25519");
    let new_fingerprint = migrated["fingerprint"].as_str().unwrap().to_string();

    // migration_complete persisted the keys this rotation moved to, so a later
    // chain walk verifies this hop from the row alone — the value the "" column
    // default would silently leave empty if the writer ever stopped setting it.
    let transition = entity::key_transitions::Entity::find()
        .filter(entity::key_transitions::Column::OldFingerprint.eq(user.rsa_fingerprint.clone()))
        .one(&context.db)
        .await
        .unwrap()
        .expect("a transition row was written");
    assert_eq!(transition.new_identity_key_pem, migrated["pubkey"].as_str().unwrap());
    assert_eq!(
        transition.new_wrapping_key_pem,
        migrated["wrapping_pubkey"].as_str().unwrap()
    );

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

    let user = register_legacy_with(&app, &context.db, "migrate-second@example.com").await;
    create_file(&app, &user.jwt).await;

    let (resp, _, _, _) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // The legacy session's JWT is still valid, but the account is no longer
    // legacy, so a replayed migration is refused at both steps: staging a
    // re-wrap and completing again.
    let parts = build_migration_parts(&app, &user).await;
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/rewrap")
        .cookie(user.jwt.clone())
        .set_json(json!({ "keys": parts.rewrap_keys }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::BAD_REQUEST,
        "staging a re-wrap after migration is refused"
    );

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::BAD_REQUEST,
        "completing after migration is refused"
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

    let user = register_legacy_with(&app, &context.db, "migrate-siglogin@example.com").await;
    let _file_id = create_file(&app, &user.jwt).await;

    let (migrate_resp, _x_private, _file_key, _link_rewraps) = migrate(&app, &user).await;
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
            ..Default::default()
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

    // The client-nonce format must resolve through the same transition
    // fallback: the canonical is built from the presented (old) fingerprint.
    let timestamp = chrono::Utc::now().timestamp();
    let nonce = entity::Uuid::new_v4().simple().to_string();
    let canonical = format!("{old_fp}:{timestamp}:{nonce}");
    let signature = cryptfns::rsa::private::sign(&canonical, &user.rsa_private).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .set_json(&Signature {
            fingerprint: Some(old_fp.clone()),
            signature: Some(signature),
            timestamp: Some(timestamp),
            nonce: Some(nonce),
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "client-nonce signature login with the old fingerprint must succeed via key transition"
    );
}

/// A migrating owner's public link key is wrapped under their old RSA key, so
/// migration must re-wrap it under the new X25519 key alongside the file keys —
/// otherwise the owner loses access to every pre-migration link they created.
#[actix_web::test]
async fn test_migration_rewraps_owned_public_link_key() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-rewraplink@example.com").await;
    let file_id = create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;
    let link_id = seed_link(&context.db, user_id, file_id, "old-rsa-link-key").await;

    let (resp, _x_private, _file_key, link_rewraps) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK, "migration completes");

    let sent = link_rewraps
        .iter()
        .find(|(id, _)| *id == link_id)
        .map(|(_, value)| value.clone())
        .expect("the owned link was offered for re-wrap");

    let row = entity::links::Entity::find_by_id(link_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.encrypted_link_key, sent,
        "the link key was re-wrapped to the value the client submitted"
    );
    assert_ne!(
        row.encrypted_link_key, "old-rsa-link-key",
        "the pre-migration RSA-wrapped link key must not survive"
    );
}

/// A `rewrap` batch naming a link the caller does not own is rejected at staging,
/// so a foreign id never enters the migrator's set: the victim's row is untouched
/// and the migrator, having staged nothing, stays legacy.
#[actix_web::test]
async fn test_migration_aborts_on_foreign_link_key() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // The migrating account, with a file so we can prove its own key is untouched.
    let user = register_legacy_with(&app, &context.db, "migrate-foreignlink@example.com").await;
    let file_id = create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;

    // A second account owning a link the migrator does not. Its link reuses the
    // migrator's file id (any existing file satisfies the FK); ownership is on
    // `links.user_id`, which is what the server checks.
    let other = helpers::seed_legacy_user(&context.db, "other@example.com").await;
    let foreign_link = seed_link(&context.db, other.user_id, file_id, "victim-link-key").await;

    // Staging a link the caller does not own is rejected outright, so a foreign
    // id never lands in the migrator's set to begin with.
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/rewrap")
        .cookie(user.jwt.clone())
        .set_json(json!({
            "link_keys": [
                { "link_id": foreign_link, "encrypted_link_key": "attacker-rewrap", "signature": "irrelevant" }
            ]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "rewrapped_link_key_not_owned");

    // Nothing was applied: the account is still legacy, its own file key was not
    // re-wrapped, and no staging row was created for the foreign link.
    assert_eq!(staging_count(&context.db, other.user_id).await, 0);
    let migrator = entity::users::Entity::find_by_id(user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrator.security_version, 0, "account stays legacy after abort");

    let file_row = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::FileId.eq(file_id))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        file_row.encrypted_key, "old-rsa-wrapped-key",
        "the migrator's own file key must be untouched"
    );

    // The victim's link is intact.
    let victim = entity::links::Entity::find_by_id(foreign_link)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        victim.encrypted_link_key, "victim-link-key",
        "the victim's link key must be untouched"
    );
}

/// A file staged for re-wrap and then deleted before completion — an abandoned
/// first attempt whose file the user dropped — must not block the migration. Its
/// stale staging row is skipped rather than aborting the whole ceremony (foreign
/// ids are already rejected at stage time, so a 0-row update here can only be a
/// since-deleted own file), and the account still migrates.
#[actix_web::test]
async fn test_migration_skips_staged_row_for_a_since_deleted_file() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-deleted@example.com").await;
    let user_id = current_user_id(&app, &user.jwt).await;
    helpers::seed_owned_files(&context.db, user_id, 3).await;

    let parts = build_migration_parts(&app, &user).await;
    assert_eq!(parts.rewrap_keys.len(), 3);
    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;
    assert_eq!(staging_count(&context.db, user_id).await, 3);

    // Drop one staged file, as if it was deleted between an abandoned attempt and
    // this completion; its staging row is younger than the TTL so it survives.
    let deleted_file_id: entity::Uuid = parts.rewrap_keys[0]["file_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    entity::user_files::Entity::delete_many()
        .filter(entity::user_files::Column::UserId.eq(user_id))
        .filter(entity::user_files::Column::FileId.eq(deleted_file_id))
        .exec(&context.db)
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "the migration completes, skipping the stale row"
    );

    let migrator = entity::users::Entity::find_by_id(user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrator.security_version, 1, "the account migrated");
    assert_eq!(
        staging_count(&context.db, user_id).await,
        0,
        "the stale staging row is cleared with the rest"
    );

    let untouched = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::UserId.eq(user_id))
        .filter(entity::user_files::Column::EncryptedKey.eq("old-rsa-wrapped-key"))
        .count(&context.db)
        .await
        .unwrap();
    assert_eq!(untouched, 0, "the surviving files migrated");
}

/// Seed a prior audit row on `user_id`'s sender chain so the key-rotation event
/// has a predecessor to chain off. Returns the row's `this_event_hash`.
async fn seed_prior_event<C: entity::ConnectionTrait>(
    db: &C,
    user_id: entity::Uuid,
    file_id: entity::Uuid,
) -> Vec<u8> {
    let hash = vec![0xABu8; 32];
    share_events::Entity::insert(share_events::ActiveModel {
        id: ActiveValue::Set(entity::Uuid::now_v7()),
        sender_id: ActiveValue::Set(Some(user_id)),
        recipient_id: ActiveValue::Set(None),
        file_id: ActiveValue::Set(Some(file_id)),
        action: ActiveValue::Set("grant".to_string()),
        share_role_before: ActiveValue::Set(None),
        share_role_after: ActiveValue::Set(Some("reader".to_string())),
        created_at: ActiveValue::Set(chrono::Utc::now().timestamp() - 100),
        prev_event_hash: ActiveValue::Set(None),
        this_event_hash: ActiveValue::Set(hash.clone()),
        sender_signature: ActiveValue::Set(None),
    })
    .exec_without_returning(db)
    .await
    .unwrap();
    hash
}

/// Migration appends a `key_rotation` event that chains off the sender's prior
/// event and whose signature verifies against the new identity key over the
/// canonical the server rebuilds from its own state.
#[actix_web::test]
async fn test_migration_appends_verifying_key_rotation_event() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-keyrot@example.com").await;
    let file_id = create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;
    let prior_hash = seed_prior_event(&context.db, user_id, file_id).await;

    let (resp, _x, _fk, _l) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let migrated: Value = test::read_body_json(resp).await;
    let new_pubkey = migrated["pubkey"].as_str().unwrap();
    let new_fingerprint = migrated["fingerprint"].as_str().unwrap();

    let row = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("key_rotation"))
        .one(&context.db)
        .await
        .unwrap()
        .expect("a key_rotation row was appended");
    assert_eq!(row.sender_id, Some(user_id));
    assert!(row.recipient_id.is_none());
    assert!(row.file_id.is_none(), "a key rotation belongs to no file");
    assert_eq!(
        row.prev_event_hash.as_deref(),
        Some(prior_hash.as_slice()),
        "the event chains off the seeded prior row"
    );

    let audit_row = cryptfns::asn1::AuditEventRowV1 {
        sender_id: user_id.into_bytes(),
        recipient_id: [0u8; 16],
        file_id: [0u8; 16],
        action: "key_rotation".to_string(),
        share_role: None,
        created_at: row.created_at,
    };
    let recomputed = cryptfns::asn1::audit_event_chain_hash(
        prior_hash.as_slice().try_into().unwrap(),
        &audit_row,
    )
    .unwrap();
    assert_eq!(row.this_event_hash, recomputed.to_vec(), "chain hash matches");

    let sig_b64 = cryptfns::base64::encode(row.sender_signature.as_ref().unwrap());
    cryptfns::transition::verify_key_rotation_audit(
        &user_id.into_bytes(),
        &user.rsa_fingerprint,
        new_fingerprint,
        row.created_at,
        &sig_b64,
        new_pubkey,
    )
    .expect("the stored signature verifies under the new identity key");
}

/// A bad key-rotation audit signature aborts the whole migration before any
/// state changes: the account stays legacy and no event is written.
#[actix_web::test]
async fn test_migration_aborts_on_bad_audit_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-badaudit@example.com").await;
    create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;

    let parts = build_migration_parts(&app, &user).await;
    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;

    let mut body = parts.complete_body;
    let sig = body["audit_event_signature"].as_str().unwrap().to_string();
    let mut bytes = cryptfns::base64::decode(&sig).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    body["audit_event_signature"] = json!(cryptfns::base64::encode(&bytes));

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "audit_event_signature_invalid");

    let migrator = entity::users::Entity::find_by_id(user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrator.security_version, 0, "account stays legacy");
    let events = share_events::Entity::find().all(&context.db).await.unwrap();
    assert!(events.is_empty(), "no audit row written on abort");
    // The rejection is pre-transaction, so the staged re-wraps survive for a
    // retry rather than being consumed by the failed completion.
    assert!(
        staging_count(&context.db, user_id).await > 0,
        "staged re-wraps are preserved when completion is rejected"
    );
}

/// A migrated link carries the owner's re-signature over its file_id under the
/// new identity key, so link verification passes post-migration instead of
/// tripping on a stale RSA signature.
#[actix_web::test]
async fn test_migration_resigns_link_under_new_key() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-linksig@example.com").await;
    let file_id = create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;
    let link_id = seed_link(&context.db, user_id, file_id, "old-rsa-link-key").await;

    let (resp, _x, _fk, _l) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let migrated: Value = test::read_body_json(resp).await;
    let new_pubkey = migrated["pubkey"].as_str().unwrap();

    let row = entity::links::Entity::find_by_id(link_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(row.signature, "link-signature", "the RSA-era signature is gone");
    cryptfns::identity::KeyType::Curve25519
        .verify(&file_id.to_string(), &row.signature, new_pubkey)
        .expect("the stored link signature verifies under the new identity key");
}

/// A link re-signature that does not verify under the new key aborts the whole
/// migration: the account stays legacy and the link is untouched.
#[actix_web::test]
async fn test_migration_aborts_on_bad_link_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-badlink@example.com").await;
    let file_id = create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;
    let link_id = seed_link(&context.db, user_id, file_id, "old-rsa-link-key").await;

    let mut parts = build_migration_parts(&app, &user).await;
    // The ceremony re-signed the link correctly; corrupt just that signature
    // before staging. `rewrap` does not verify signatures — the new identity key
    // is not committed until `complete`, which is where the check fires.
    let entry = parts
        .rewrap_link_keys
        .iter_mut()
        .find(|e| e["link_id"] == link_id.to_string())
        .expect("the owned link is in the ceremony");
    let sig = entry["signature"].as_str().unwrap().to_string();
    let mut bytes = cryptfns::base64::decode(&sig).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    entry["signature"] = json!(cryptfns::base64::encode(&bytes));

    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "link_signature_invalid");

    let migrator = entity::users::Entity::find_by_id(user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrator.security_version, 0, "account stays legacy");
    let row = entity::links::Entity::find_by_id(link_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.encrypted_link_key, "old-rsa-link-key", "link untouched");
}

/// The regression test for the payload bug: an account with more files than the
/// pre-fix single `migration/complete` POST could carry (its body would exceed
/// actix's 2 MB `Json` limit and 413) migrates successfully through the batched
/// `rewrap` staging. We assert the old body would indeed have been over the
/// limit, then prove every key is re-wrapped anyway.
#[actix_web::test]
async fn test_large_account_that_overflowed_the_old_post_migrates() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-large@example.com").await;
    let user_id = current_user_id(&app, &user.jwt).await;

    // Above the ~9.6k-file break point where the old single POST hit 413.
    const FILES: usize = 10_000;
    helpers::seed_owned_files(&context.db, user_id, FILES).await;

    let parts = build_migration_parts(&app, &user).await;
    assert_eq!(parts.rewrap_keys.len(), FILES, "every seeded file is offered for re-wrap");

    // The body the pre-fix client would have sent in one shot — prove it clears
    // 2 MB, which is exactly what made the old design 413 for this account.
    let mut old_single_post = parts.complete_body.clone();
    old_single_post["rewrapped_keys"] = json!(parts.rewrap_keys);
    old_single_post["rewrapped_link_keys"] = json!(parts.rewrap_link_keys);
    let old_size = serde_json::to_vec(&old_single_post).unwrap().len();
    assert!(
        old_size > 2 * 1024 * 1024,
        "the pre-fix single POST ({old_size} B) would have exceeded actix's 2 MB Json limit"
    );

    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "the large account migrates");

    let untouched = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::UserId.eq(user_id))
        .filter(entity::user_files::Column::EncryptedKey.eq("old-rsa-wrapped-key"))
        .count(&context.db)
        .await
        .unwrap();
    assert_eq!(untouched, 0, "no file key was left un-migrated");
    assert_eq!(
        staging_count(&context.db, user_id).await,
        0,
        "staging is cleared once complete applies it"
    );

    // A re-wrapped key really does unwrap under the new key to the file key sent.
    let sample = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::UserId.eq(user_id))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    let recovered = cryptfns::ecdh::unwrap(&sample.encrypted_key, &parts.x_private).unwrap();
    assert_eq!(recovered, parts.file_key, "file key survives the batched re-wrap");
}

/// A 600-file account needs more than one `rewrap` batch (the per-request cap is
/// 500), so it exercises staging accumulating across batches before a single
/// completion applies the whole set.
#[actix_web::test]
async fn test_account_migrates_across_multiple_rewrap_batches() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-multibatch@example.com").await;
    let user_id = current_user_id(&app, &user.jwt).await;
    helpers::seed_owned_files(&context.db, user_id, 600).await;

    let (resp, _x, _fk, _l) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let untouched = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::UserId.eq(user_id))
        .filter(entity::user_files::Column::EncryptedKey.eq("old-rsa-wrapped-key"))
        .count(&context.db)
        .await
        .unwrap();
    assert_eq!(untouched, 0, "all 600 keys re-wrapped across the batches");
}

/// A `rewrap` batch larger than the per-request cap is rejected with a clear
/// error rather than being allowed to grow the body unbounded. The cap is
/// checked before ownership, so unowned stand-in ids are enough to trip it.
#[actix_web::test]
async fn test_oversized_rewrap_batch_is_rejected() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-toobig@example.com").await;

    let keys: Vec<Value> = (0..501)
        .map(|_| json!({ "file_id": entity::Uuid::new_v4(), "encrypted_key": "x" }))
        .collect();
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/rewrap")
        .cookie(user.jwt.clone())
        .set_json(json!({ "keys": keys }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "rewrap_batch_too_large");
}

/// Re-staging the same batch — a normal outcome of a retried request — replaces
/// its rows instead of duplicating them, so the applied set never doubles.
#[actix_web::test]
async fn test_rewrap_batch_is_idempotent() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-idempotent@example.com").await;
    let user_id = current_user_id(&app, &user.jwt).await;
    helpers::seed_owned_files(&context.db, user_id, 3).await;

    let parts = build_migration_parts(&app, &user).await;

    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;
    assert_eq!(staging_count(&context.db, user_id).await, 3, "first stage lands 3 rows");

    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;
    assert_eq!(
        staging_count(&context.db, user_id).await,
        3,
        "a retried batch replaces, never duplicates"
    );

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);
    assert_eq!(staging_count(&context.db, user_id).await, 0);

    let untouched = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::UserId.eq(user_id))
        .filter(entity::user_files::Column::EncryptedKey.eq("old-rsa-wrapped-key"))
        .count(&context.db)
        .await
        .unwrap();
    assert_eq!(untouched, 0);
}

/// A migrated account has no legacy keys left to re-wrap, so a stray `rewrap`
/// after completion is refused.
#[actix_web::test]
async fn test_rewrap_rejected_after_migration() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-postrewrap@example.com").await;
    create_file(&app, &user.jwt).await;

    let (resp, _x, _fk, _l) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/rewrap")
        .cookie(user.jwt.clone())
        .set_json(json!({ "keys": [{ "file_id": entity::Uuid::new_v4(), "encrypted_key": "x" }] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "already_migrated");
}

/// An account with no files and no links stages nothing, and `complete` with an
/// empty staging set still flips it to the migrated identity.
#[actix_web::test]
async fn test_complete_with_empty_staging_succeeds() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-empty@example.com").await;
    let user_id = current_user_id(&app, &user.jwt).await;

    let (resp, _x, _fk, _l) = migrate(&app, &user).await;
    assert_eq!(resp.status(), StatusCode::OK, "a zero-file account still migrates");
    let migrated: Value = test::read_body_json(resp).await;
    assert_eq!(migrated["security_version"], 1);
    assert_eq!(staging_count(&context.db, user_id).await, 0);
}

/// Staging abandoned by a migration the client never finished is purged by TTL
/// on the next `rewrap`, so it cannot leak into — and abort — a later migration
/// by the same user.
#[actix_web::test]
async fn test_abandoned_staging_is_purged_and_does_not_leak() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-abandoned@example.com").await;
    create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;

    // A stale row from an abandoned prior ceremony, for a file that is no longer
    // re-staged. Left in place it would make `complete` roll back (its UPDATE
    // matches nothing); it must be purged before the fresh migration applies.
    migration_rewrap_staging::Entity::insert(migration_rewrap_staging::ActiveModel {
        id: ActiveValue::Set(entity::Uuid::new_v4()),
        user_id: ActiveValue::Set(user_id),
        file_id: ActiveValue::Set(Some(entity::Uuid::new_v4())),
        link_id: ActiveValue::Set(None),
        encrypted_key: ActiveValue::Set("stale-abandoned-blob".to_string()),
        signature: ActiveValue::Set(None),
        created_at: ActiveValue::Set(chrono::Utc::now().timestamp() - 2 * 24 * 60 * 60),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();
    assert_eq!(staging_count(&context.db, user_id).await, 1);

    let (resp, _x, _fk, _l) = migrate(&app, &user).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "the fresh migration succeeds; the stale row was purged, not applied"
    );
    assert_eq!(staging_count(&context.db, user_id).await, 0);
}

/// A failure between staging and completion (here a rejected completion standing
/// in for a crash) leaves the account fully legacy with its keys untouched and
/// the staging intact — and a retry of `complete` then succeeds from that same
/// staging.
#[actix_web::test]
async fn test_retry_after_failure_between_staging_and_complete() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let user = register_legacy_with(&app, &context.db, "migrate-retry@example.com").await;
    let file_id = create_file(&app, &user.jwt).await;
    let user_id = current_user_id(&app, &user.jwt).await;

    let parts = build_migration_parts(&app, &user).await;
    stage_rewraps(&app, &user, &parts.rewrap_keys, &parts.rewrap_link_keys).await;

    // First completion fails after staging (bad audit signature stands in for a
    // crash). The account stays legacy, its key is untouched, staging survives.
    let mut bad = parts.complete_body.clone();
    let sig = bad["audit_event_signature"].as_str().unwrap().to_string();
    let mut bytes = cryptfns::base64::decode(&sig).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    bad["audit_event_signature"] = json!(cryptfns::base64::encode(&bytes));

    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&bad)
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::BAD_REQUEST
    );

    let legacy = entity::users::Entity::find_by_id(user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(legacy.security_version, 0, "account stays legacy after the failure");
    let file_row = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::FileId.eq(file_id))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(file_row.encrypted_key, "old-rsa-wrapped-key", "key untouched");
    assert!(staging_count(&context.db, user_id).await > 0, "staging survives for the retry");

    // The retry completes from the same staged set.
    let req = test::TestRequest::post()
        .uri("/api/auth/migration/complete")
        .cookie(user.jwt.clone())
        .set_json(&parts.complete_body)
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

    let migrated = entity::users::Entity::find_by_id(user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(migrated.security_version, 1, "the retry migrates");
    assert_eq!(staging_count(&context.db, user_id).await, 0);
    let file_row = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::FileId.eq(file_id))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(file_row.encrypted_key, "old-rsa-wrapped-key", "key re-wrapped on retry");
}
