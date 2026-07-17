//! Password change for a v2 (Curve25519 + OPAQUE) account. The client
//! re-registers OPAQUE under the new password and re-seals its private-key
//! envelope under the new `export_key`; the server commits the new password
//! file and the new envelope together. A session cookie is not enough to
//! authorize the change — the request must carry a signature by the account's
//! identity key (plus TOTP when 2FA is on), so a stolen cookie cannot rotate
//! the password or overwrite the envelope. After a valid change the old
//! password no longer logs in, the new one does, and the new login's
//! `export_key` still opens the stored envelope — the keys are recoverable, not
//! stranded.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test};
use hoodik::server;
use serde_json::{json, Value};

const EMAIL: &str = "rotate@example.com";
const OLD_PASSWORD: &[u8] = helpers::LEGACY_PASSWORD.as_bytes();
const NEW_PASSWORD: &[u8] = b"an-entirely-different-passphrase-42";

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

/// A v2 account together with the identity private key (to sign the ownership
/// proof) and the plaintext key bundle (to re-seal under the new password).
struct Account {
    jwt: actix_web::cookie::Cookie<'static>,
    user_id: entity::Uuid,
    ed_private: String,
    bundle: Vec<u8>,
}

/// Register a v2 (Curve25519 + OPAQUE) account through the real endpoints,
/// keeping the identity private key and the plaintext bundle — neither of which
/// [`helpers::register_curve25519`] exposes, but both of which a password change
/// needs to prove and re-seal.
async fn register_account(app: &impl TestApp, email: &str) -> Account {
    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_private = cryptfns::ecdh::private::generate().unwrap();
    let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();

    let reg_start = cryptfns::opaque::client_registration_start(OLD_PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/register/pake/start")
        .set_json(json!({ "email": email, "registration_request": reg_start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(app, req).await).await;
    let reg_finish = cryptfns::opaque::client_registration_finish(
        &reg_start.state,
        body["registration_response"].as_str().unwrap(),
        OLD_PASSWORD,
    )
    .unwrap();

    let bundle = b"v1|ed:test-identity-private|x:test-wrapping-private".to_vec();
    let export_key = cryptfns::base64::decode(&reg_finish.export_key).unwrap();
    let kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope = cryptfns::envelope::seal(&kek, &bundle).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "email": email,
            "pubkey": ed_public,
            "wrapping_pubkey": x_public,
            "fingerprint": fingerprint,
            "key_type": "curve25519",
            "encrypted_private_key": envelope,
            "opaque_registration_upload": reg_finish.message,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert!(resp.status().is_success(), "register failed: {:?}", resp.status());
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.expect("register response missing JWT cookie");
    let body: Value = test::read_body_json(resp).await;
    let user_id = entity::Uuid::parse_str(body["user"]["id"].as_str().unwrap()).unwrap();

    Account { jwt, user_id, ed_private, bundle }
}

/// Run the client half of a v2 password change: OPAQUE-register `NEW_PASSWORD`
/// and re-seal the account's key bundle under its new `export_key`. Returns the
/// registration upload and the sealed envelope — the two artifacts the finish
/// request carries. Does not sign or POST, so tests can vary the proof.
async fn prepare_change(app: &impl TestApp, account: &Account) -> (String, String) {
    let start = cryptfns::opaque::client_registration_start(NEW_PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/start")
        .cookie(account.jwt.clone())
        .set_json(json!({ "registration_request": start.message }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;

    let finish = cryptfns::opaque::client_registration_finish(
        &start.state,
        body["registration_response"].as_str().unwrap(),
        NEW_PASSWORD,
    )
    .unwrap();

    let export_key = cryptfns::base64::decode(&finish.export_key).unwrap();
    let new_kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope = cryptfns::envelope::seal(&new_kek, &account.bundle).unwrap();

    (finish.message, envelope)
}

/// The canonical the ownership signature commits to. Kept identical to the
/// server's `PAKE_REGISTER_CANONICAL_PREFIX` construction — this literal is the
/// wire contract both halves must agree on byte for byte.
fn change_canonical(registration_upload: &str, issued_at: i64) -> String {
    format!("hoodik-pake-register-v1\0{registration_upload}\0{issued_at}")
}

/// POST the finish request with explicit fields so rejection tests can vary the
/// signature, timestamp, and token independently.
async fn post_finish(
    app: &impl TestApp,
    jwt: &actix_web::cookie::Cookie<'static>,
    registration_upload: &str,
    envelope: &str,
    signature: &str,
    issued_at: i64,
    token: Option<&str>,
) -> ServiceResponse<EitherBody<BoxBody>> {
    let mut body = json!({
        "registration_upload": registration_upload,
        "encrypted_private_key": envelope,
        "signature": signature,
        "issued_at": issued_at,
    });
    if let Some(token) = token {
        body["token"] = json!(token);
    }

    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/finish")
        .cookie(jwt.clone())
        .set_json(&body)
        .to_request();
    test::call_service(app, req).await
}

/// Enable TOTP on an account by setting a secret directly, so the 2FA gate on
/// the password change has something to enforce.
async fn enable_two_factor(db: &entity::DbConn, user_id: entity::Uuid) {
    use entity::{ActiveModelTrait, ActiveValue};

    entity::users::ActiveModel {
        id: ActiveValue::Set(user_id),
        secret: ActiveValue::Set(Some("JBSWY3DPEHPK3PXP".to_string())),
        ..Default::default()
    }
    .update(db)
    .await
    .unwrap();
}

/// Drive the two-step OPAQUE login. A wrong password can't complete the client
/// KE, so `client_login_finish` returns `None`; otherwise return the server's
/// finish response together with the login's `export_key`.
async fn opaque_login(
    app: &impl TestApp,
    password: &[u8],
) -> Option<(ServiceResponse<EitherBody<BoxBody>>, Vec<u8>)> {
    let start = cryptfns::opaque::client_login_start(password).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .set_json(json!({ "email": EMAIL, "credential_request": start.message }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["method"], "opaque", "v2 account must offer OPAQUE");

    let login_id = body["login_id"].as_str().unwrap();
    let credential_response = body["credential_response"].as_str().unwrap();

    let finish =
        cryptfns::opaque::client_login_finish(&start.state, credential_response, password).ok()?;

    let req = test::TestRequest::post()
        .uri("/api/auth/login/finish")
        .set_json(json!({
            "login_id": login_id,
            "credential_finalization": finish.finalization,
        }))
        .to_request();
    let export_key = cryptfns::base64::decode(&finish.export_key).unwrap();
    Some((test::call_service(app, req).await, export_key))
}

async fn stored_envelope(db: &entity::DbConn, user_id: entity::Uuid) -> String {
    use entity::EntityTrait;
    entity::users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
        .encrypted_private_key
        .unwrap()
}

/// Sign the ownership proof with the account's identity key at `now`.
fn sign_now(account: &Account, registration_upload: &str) -> (String, i64) {
    let issued_at = chrono::Utc::now().timestamp();
    let signature =
        cryptfns::ed25519::private::sign(&change_canonical(registration_upload, issued_at), &account.ed_private)
            .unwrap();
    (signature, issued_at)
}

#[actix_web::test]
async fn v2_password_change_rotates_login_and_rekeys_envelope() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let account = register_account(&app, EMAIL).await;

    let (upload, sealed) = prepare_change(&app, &account).await;
    let (signature, issued_at) = sign_now(&account, &upload);
    let resp = post_finish(&app, &account.jwt, &upload, &sealed, &signature, issued_at, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "a signed change with 2FA off succeeds"
    );

    assert_eq!(
        stored_envelope(&context.db, account.user_id).await,
        sealed,
        "server stored the client's re-sealed envelope verbatim"
    );

    // The old password no longer authenticates.
    match opaque_login(&app, OLD_PASSWORD).await {
        None => {}
        Some((resp, _)) => assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "the old password must not log in after a change"
        ),
    }

    // The new password logs in, and its export_key opens the stored envelope,
    // recovering the exact bundle — the keys are not stranded.
    let (resp, export_key) = opaque_login(&app, NEW_PASSWORD)
        .await
        .expect("the new password completes the client KE");
    assert_eq!(resp.status(), StatusCode::OK, "the new password logs in via OPAQUE");

    let kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let recovered = cryptfns::envelope::open(&kek, &sealed).unwrap();
    assert_eq!(
        recovered, account.bundle,
        "the new export_key re-derives a KEK that recovers the private-key bundle"
    );
}

#[actix_web::test]
async fn v2_password_change_requires_valid_ownership_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let account = register_account(&app, EMAIL).await;
    let before = stored_envelope(&context.db, account.user_id).await;
    let issued_at = chrono::Utc::now().timestamp();

    // No signature at all.
    let (upload, sealed) = prepare_change(&app, &account).await;
    let resp = post_finish(&app, &account.jwt, &upload, &sealed, "", issued_at, None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "an empty signature proves nothing");
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "ownership_proof_required");

    // A signature by a key that is not the account's identity key.
    let attacker = cryptfns::ed25519::private::generate().unwrap();
    let (upload, sealed) = prepare_change(&app, &account).await;
    let forged =
        cryptfns::ed25519::private::sign(&change_canonical(&upload, issued_at), &attacker).unwrap();
    let resp = post_finish(&app, &account.jwt, &upload, &sealed, &forged, issued_at, None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "a signature by the wrong key is refused");

    // A signature over a different upload than the one submitted: the server
    // rebuilds the canonical from the request's own upload, so the proof must
    // bind to it.
    let (upload, sealed) = prepare_change(&app, &account).await;
    let mismatched =
        cryptfns::ed25519::private::sign(&change_canonical("a-different-upload", issued_at), &account.ed_private)
            .unwrap();
    let resp = post_finish(&app, &account.jwt, &upload, &sealed, &mismatched, issued_at, None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "the signature must bind to the actual upload");

    assert_eq!(
        stored_envelope(&context.db, account.user_id).await,
        before,
        "no rejected attempt altered the stored envelope"
    );
}

#[actix_web::test]
async fn v2_password_change_rejects_stale_timestamp() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let account = register_account(&app, EMAIL).await;
    let before = stored_envelope(&context.db, account.user_id).await;

    // Correctly signed, but the timestamp is well outside the ±300s window.
    let (upload, sealed) = prepare_change(&app, &account).await;
    let issued_at = chrono::Utc::now().timestamp() - 600;
    let signature =
        cryptfns::ed25519::private::sign(&change_canonical(&upload, issued_at), &account.ed_private).unwrap();
    let resp = post_finish(&app, &account.jwt, &upload, &sealed, &signature, issued_at, None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "signature_timestamp_skew");

    assert_eq!(
        stored_envelope(&context.db, account.user_id).await,
        before,
        "a stale request changes nothing"
    );
}

#[actix_web::test]
async fn v2_password_change_requires_totp_when_2fa_enabled() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let account = register_account(&app, EMAIL).await;
    enable_two_factor(&context.db, account.user_id).await;
    let before = stored_envelope(&context.db, account.user_id).await;

    // A valid signature and timestamp, but no TOTP token — the 2FA gate must
    // reject the change before it lands.
    let (upload, sealed) = prepare_change(&app, &account).await;
    let (signature, issued_at) = sign_now(&account, &upload);
    let resp = post_finish(&app, &account.jwt, &upload, &sealed, &signature, issued_at, None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "invalid_otp_token");

    assert_eq!(
        stored_envelope(&context.db, account.user_id).await,
        before,
        "a missing TOTP token changes nothing"
    );
}

#[actix_web::test]
async fn v2_password_change_rejects_legacy_account() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    // A legacy (v0) account with a valid session. It must migrate before it can
    // change its password — planting an OPAQUE password file here would leave a
    // bcrypt hash and an OPAQUE file side by side, the hybrid state migration
    // forbids.
    helpers::seed_legacy_user(&context.db, "legacy@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "legacy@example.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // The version gate fires before any signature or OPAQUE work, so dummy
    // artifacts are enough to reach it.
    let issued_at = chrono::Utc::now().timestamp();
    let resp = post_finish(&app, &jwt, "upload", "envelope", "signature", issued_at, None).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let msg: Value = test::read_body_json(resp).await;
    assert_eq!(msg["message"], "password_change_requires_migration");
}
