#[path = "./helpers.rs"]
mod helpers;

use std::{str::FromStr, time::Duration};

use actix_web::{cookie::Expiration, http::StatusCode, test};
use auth::{
    data::{authenticated::Authenticated, credentials::Credentials, signature::Signature},
    mock::generate_fingerprint_nonce,
};
use context::SenderContract;
use hoodik::server;

#[actix_web::test]
async fn test_registration_and_login() {
    let context = context::Context::mock_sqlite().await;

    let app = test::init_service(server::app(context.clone())).await;

    // Password + RSA-signature login are the pre-migration paths, so this drives
    // a seeded legacy account through both.
    let owner = helpers::seed_legacy_user(&context.db, "john@doe.com").await;
    let fingerprint = owner.rsa_fingerprint.clone();
    let private_string = owner.rsa_private.clone();

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(&Credentials {
            email: Some("john@doe.com".to_string()),
            password: Some(helpers::LEGACY_PASSWORD.to_string()),
            token: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let nonce = generate_fingerprint_nonce(&fingerprint);
    let signature = cryptfns::rsa::private::sign(&nonce, &private_string).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .set_json(&Signature {
            fingerprint: Some(fingerprint.clone()),
            signature: Some(signature),
            ..Default::default()
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, refresh) = helpers::extract_cookies(resp.headers());

    assert_eq!(resp.status(), StatusCode::OK);
    // println!("{:?}", &resp.headers());

    let _response: Authenticated = serde_json::from_slice(&test::read_body(resp).await).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .cookie(jwt.unwrap())
        .cookie(refresh.unwrap())
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();

    let (jwt, _refresh) = helpers::extract_cookies(resp.headers());

    // let body = test::read_body(resp).await;
    // let body_str = String::from_utf8_lossy(&body).to_string();
    // println!("{:#?}", body_str);

    assert_eq!(status, StatusCode::OK);

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();

    assert_eq!(status, StatusCode::OK);

    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::call_service(&app, req).await;
    let status = resp.status();
    let (jwt, refresh) = helpers::extract_cookies(resp.headers());

    // let body = test::read_body(resp).await;
    // let body_str = String::from_utf8_lossy(&body).to_string();
    // println!("{:#?}", body_str);

    assert_eq!(status, StatusCode::NO_CONTENT);

    match jwt.unwrap().expires().unwrap() {
        Expiration::DateTime(dt) => {
            assert!(dt.unix_timestamp() < chrono::Utc::now().timestamp());
        }
        _ => panic!("Expected DateTime"),
    }

    match refresh.unwrap().expires().unwrap() {
        Expiration::DateTime(dt) => {
            assert!(dt.unix_timestamp() < chrono::Utc::now().timestamp());
        }
        _ => panic!("Expected DateTime"),
    }
}

#[actix_web::test]
async fn test_register_and_verify_user_email() {
    let context = context::Context::add_mock_sender(context::Context::mock_sqlite().await);

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let id = context
        .sender
        .as_ref()
        .unwrap()
        .find("Account activation token:")
        .unwrap()
        .replace("Account activation token: ", "");

    let _uuid = entity::Uuid::from_str(&id).unwrap();

    let req = test::TestRequest::post()
        .uri(format!("/api/auth/action/activate-email/{id}").as_str())
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&body).to_string();
    let user = serde_json::from_str::<entity::users::Model>(&body_str).unwrap();

    assert!(user.email_verified_at.is_some());
}

#[actix_web::test]
async fn test_claims_can_expire() {
    let mut context = context::Context::mock_sqlite().await;
    context.config.auth.short_term_session_duration_seconds = 1;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());

    assert_eq!(resp.status(), StatusCode::CREATED);

    std::thread::sleep(Duration::from_secs(2));

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_expired_session_can_be_refreshed() {
    let mut context = context::Context::mock_sqlite().await;
    context.config.auth.short_term_session_duration_seconds = 1;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, refresh) = helpers::extract_cookies(resp.headers());

    assert_eq!(resp.status(), StatusCode::CREATED);

    std::thread::sleep(Duration::from_secs(2));

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .cookie(jwt.clone().unwrap())
        .cookie(refresh.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();
    let status = resp.status();
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    // let body = test::read_body(resp).await;
    // let body_str = String::from_utf8_lossy(&body).to_string();
    // println!("{:#?}", body_str);

    assert_eq!(status, StatusCode::OK);

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn cannot_refresh_logged_out_session() {
    let context = context::Context::mock_sqlite().await;

    let app = test::init_service(server::app(context.clone())).await;

    let body = helpers::build_curve25519_register_body(&app, "john@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&body)
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, refresh) = helpers::extract_cookies(resp.headers());

    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let req = test::TestRequest::post()
        .uri("/api/auth/logout")
        .cookie(jwt.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();
    let status = resp.status();
    // let body = test::read_body(resp).await;
    // let body_str = String::from_utf8_lossy(&body).to_string();
    // println!("{:#?}", body_str);

    assert_eq!(status, StatusCode::NO_CONTENT);

    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .cookie(jwt.clone().unwrap())
        .cookie(refresh.clone().unwrap())
        .to_request();

    let resp = test::try_call_service(&app, req).await.unwrap();
    let status = resp.status();
    // let body = test::read_body(resp).await;
    // let body_str = String::from_utf8_lossy(&body).to_string();
    // println!("{:#?}", body_str);

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
