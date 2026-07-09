//! A brand-new signup creates a v2 (Curve25519 + OPAQUE) account directly,
//! driven end to end through the real routes with `cryptfns` as the client.
//! No migration is involved — the account is born `security_version = 1`.

use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{http::StatusCode, test};
use entity::{ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use serde_json::{json, Value};

const PASSWORD: &str = "not-4-weak-password-for-god-sakes!";

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

/// Run the client half of a v2 signup: generate the curve keys, complete OPAQUE
/// registration through the unauthenticated start endpoint, envelope-seal the
/// private bundle under the export key, and POST `/api/auth/register`. Returns
/// the register response and the X25519 private key.
async fn signup_v2(
    app: &impl TestApp,
    email: &str,
    password: &str,
) -> (ServiceResponse<EitherBody<BoxBody>>, String) {
    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_private = cryptfns::ecdh::private::generate().unwrap();
    let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();

    // Unauthenticated OPAQUE registration start, keyed by the email.
    let reg_start = cryptfns::opaque::client_registration_start(password.as_bytes()).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/register/pake/start")
        .set_json(json!({ "email": email, "registration_request": reg_start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(app, req).await).await;
    let reg_response = body["registration_response"].as_str().unwrap();
    let reg_finish =
        cryptfns::opaque::client_registration_finish(&reg_start.state, reg_response, password.as_bytes())
            .unwrap();

    // Seal the private-key bundle under the KEK derived from the export key.
    let export_key = cryptfns::base64::decode(&reg_finish.export_key).unwrap();
    let kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope =
        cryptfns::envelope::seal(&kek, format!("v1|ed:{ed_private}|x:{x_private}").as_bytes())
            .unwrap();

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

    (test::call_service(app, req).await, x_private)
}

#[actix_web::test]
async fn test_v2_signup_creates_a_migrated_account_and_opaque_login_works() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;
    let email = "v2signup@example.com";

    let (resp, _x_private) = signup_v2(&app, email, PASSWORD).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "v2 signup authenticates");
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["user"]["security_version"], 1, "born migrated");
    assert_eq!(body["user"]["key_type"], "curve25519");

    // The row stores the OPAQUE password file and no bcrypt password.
    let row = entity::users::Entity::find()
        .filter(entity::users::Column::Email.eq(email))
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.security_version, 1);
    assert!(row.opaque_password_file.is_some(), "OPAQUE password file stored");
    assert!(row.password.is_none(), "no bcrypt password on a v2 account");

    // The password never crossed the wire, yet OPAQUE login completes.
    let start = cryptfns::opaque::client_login_start(PASSWORD.as_bytes()).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/start")
        .set_json(json!({ "email": email, "credential_request": start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    assert_eq!(body["method"], "opaque", "v2 account authenticates via OPAQUE");
    let login_id = body["login_id"].as_str().unwrap();
    let credential_response = body["credential_response"].as_str().unwrap();
    let finish =
        cryptfns::opaque::client_login_finish(&start.state, credential_response, PASSWORD.as_bytes())
            .unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/login/finish")
        .set_json(json!({ "login_id": login_id, "credential_finalization": finish.finalization }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::OK,
        "the same password logs in via OPAQUE"
    );
}

#[actix_web::test]
async fn test_curve_signup_requires_opaque_upload_and_forbids_password() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_public =
        cryptfns::ecdh::public::from_private(&cryptfns::ecdh::private::generate().unwrap()).unwrap();

    // A curve signup with no OPAQUE upload is rejected.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "email": "curve-no-opaque@example.com",
            "pubkey": ed_public,
            "wrapping_pubkey": x_public,
            "fingerprint": fingerprint,
            "key_type": "curve25519",
            "encrypted_private_key": "envelope",
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "curve25519 signup must carry an OPAQUE registration upload"
    );

    // A curve signup that also supplies a bcrypt password is rejected.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "email": "curve-with-password@example.com",
            "password": PASSWORD,
            "pubkey": ed_public,
            "wrapping_pubkey": x_public,
            "fingerprint": fingerprint,
            "key_type": "curve25519",
            "encrypted_private_key": "envelope",
            "opaque_registration_upload": "upload",
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "a v2 account has no bcrypt password"
    );

    // Legacy RSA registration is gone: an `rsa` (or absent) key type is refused
    // outright, so no new account can be born pre-migration.
    let rsa_private = cryptfns::rsa::private::generate().unwrap();
    let rsa_public = cryptfns::rsa::public::to_string(
        &cryptfns::rsa::public::from_private(&rsa_private).unwrap(),
    )
    .unwrap();
    let rsa_fingerprint =
        cryptfns::rsa::fingerprint(cryptfns::rsa::public::from_str(&rsa_public).unwrap()).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "email": "legacy-rsa@example.com",
            "password": PASSWORD,
            "pubkey": rsa_public,
            "fingerprint": rsa_fingerprint,
            "key_type": "rsa",
            "encrypted_private_key": "envelope",
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "RSA registration is no longer accepted"
    );
}
