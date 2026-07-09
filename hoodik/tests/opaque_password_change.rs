//! Password change for a v2 (Curve25519 + OPAQUE) account. The client
//! re-registers OPAQUE under the new password and re-seals its private-key
//! envelope under the new `export_key`; the server commits the new password
//! file and the new envelope together. After the change the old password no
//! longer logs in, the new one does, and the new login's `export_key` still
//! opens the stored envelope — proving the keys are recoverable, not stranded.

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

/// Run the client half of the v2 password change and drive both PAKE endpoints.
/// Returns the envelope the client re-sealed under the new password, so the
/// test can assert the stored one matches.
async fn change_password(
    app: &impl TestApp,
    jwt: &actix_web::cookie::Cookie<'static>,
    bundle: &[u8],
) -> String {
    let start = cryptfns::opaque::client_registration_start(NEW_PASSWORD).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/start")
        .cookie(jwt.clone())
        .set_json(json!({ "registration_request": start.message }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let registration_response = body["registration_response"].as_str().unwrap();

    let finish =
        cryptfns::opaque::client_registration_finish(&start.state, registration_response, NEW_PASSWORD)
            .unwrap();

    // Derive the KEK from the NEW password's export_key and re-seal the bundle.
    // No old KEK is needed — the client already holds the plaintext keys.
    let export_key = cryptfns::base64::decode(&finish.export_key).unwrap();
    let new_kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope = cryptfns::envelope::seal(&new_kek, bundle).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/finish")
        .cookie(jwt.clone())
        .set_json(json!({
            "registration_upload": finish.message,
            "encrypted_private_key": envelope,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    envelope
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

#[actix_web::test]
async fn v2_password_change_rotates_login_and_rekeys_envelope() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let account = helpers::register_curve25519(&app, EMAIL).await;

    // The plaintext private-key bundle the client holds in memory. The change
    // re-seals exactly these bytes under the new password.
    let bundle = b"v1|ed:test-identity-private|x:test-wrapping-private";
    let sealed = change_password(&app, &account.jwt, bundle).await;

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
        recovered, bundle,
        "the new export_key re-derives a KEK that recovers the private-key bundle"
    );
}

#[actix_web::test]
async fn v2_password_change_rejects_empty_envelope() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    let account = helpers::register_curve25519(&app, EMAIL).await;
    let before = stored_envelope(&context.db, account.user_id).await;

    let start = cryptfns::opaque::client_registration_start(NEW_PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/start")
        .cookie(account.jwt.clone())
        .set_json(json!({ "registration_request": start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(&app, req).await).await;
    let finish = cryptfns::opaque::client_registration_finish(
        &start.state,
        body["registration_response"].as_str().unwrap(),
        NEW_PASSWORD,
    )
    .unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/pake/register/finish")
        .cookie(account.jwt.clone())
        .set_json(json!({
            "registration_upload": finish.message,
            "encrypted_private_key": "   ",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "an empty envelope is a brick and must be rejected"
    );

    assert_eq!(
        stored_envelope(&context.db, account.user_id).await,
        before,
        "a rejected change leaves the original envelope untouched"
    );
}
