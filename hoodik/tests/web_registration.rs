#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use hoodik::server;
use settings::{data::Users, factory::Factory};

#[actix_web::test]
async fn test_allow_register_false() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": false,
        "enforce_email_activation": false
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let encrypted_secret = "some-random-encrypted-secret".to_string();

    let mut app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[actix_web::test]
async fn test_whitelist_pass_and_fail() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": false,
        "enforce_email_activation": false,
        "email_whitelist": {
            "rules": [
                "*@doe.com"
            ]
        }
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let encrypted_secret = "some-random-encrypted-secret".to_string();

    let mut app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@example.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[actix_web::test]
async fn test_registration_allowed_fails_blacklist_cannot_register() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": true,
        "enforce_email_activation": false,
        "email_blacklist": {
            "rules": [
                "*@doe.com"
            ]
        }
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let encrypted_secret = "some-random-encrypted-secret".to_string();

    let mut app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@example.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[actix_web::test]
async fn test_registration_not_allowed_fails_blacklist_cannot_register() {
    let context = context::Context::mock_sqlite().await;

    let mut settings = context.settings.inner().await.clone();
    settings.users = serde_json::from_value::<Users>(serde_json::json!({
        "allow_register": true,
        "enforce_email_activation": false,
        "email_blacklist": {
            "rules": [
                "*@doe.com"
            ]
        },
        "email_whitelist": {
            "rules": [
                "terry@example.com"
            ]
        }
    }))
    .unwrap();

    context.settings.replace_inner(settings).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let encrypted_secret = "some-random-encrypted-secret".to_string();

    let mut app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("terry@example.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
