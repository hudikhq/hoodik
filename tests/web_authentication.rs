use actix_web::{http::StatusCode, test};
use auth::{
    auth::Auth,
    data::{authenticated::Authenticated, create_user::CreateUser, credentials::Credentials},
};
use base64::prelude::*;
use hoodik::server;

#[actix_web::test]
async fn test_registration_and_login() {
    let context = context::Context::mock_sqlite().await;
    let auth = Auth::new(&context);

    let (public, secret) = cryptfns::generate_ed25519_keypair();

    let pubkey = cryptfns::bytes_to_mnemonic(public.as_bytes());

    let mut app = test::init_service(server::app(context.clone())).await;
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: pubkey.clone(),
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/auth/login")
        .set_json(&Credentials {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            remember: Some(true),
            token: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let authenticated: Authenticated =
        serde_json::from_slice(&test::read_body(resp).await).unwrap();

    let cookie = auth
        .manage_cookie(&authenticated.session.as_ref().unwrap(), false)
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .cookie(cookie.clone())
        .append_header((
            "X-CSRF-Token",
            authenticated.session.as_ref().unwrap().csrf.clone(),
        ))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let session: entity::sessions::Model =
        serde_json::from_slice(&test::read_body(resp).await).unwrap();

    let cookie = auth.manage_cookie(&session, false).await.unwrap();

    let req = test::TestRequest::post()
        .uri("/auth/self")
        .cookie(cookie.clone())
        .append_header(("X-CSRF-Token", session.csrf.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let message = (chrono::Utc::now().timestamp() / 60).to_string();
    let signature = cryptfns::sign(&message, secret.as_bytes(), public.as_bytes())
        .unwrap()
        .to_bytes();

    let pubkey_as_bytes = cryptfns::mnemonic_to_bytes(pubkey.as_ref().unwrap()).unwrap();

    let value = format!(
        "Signature {} {}",
        BASE64_STANDARD.encode(&signature),
        BASE64_STANDARD.encode(&pubkey_as_bytes)
    );

    let req = test::TestRequest::post()
        .uri("/auth/self")
        .append_header(("Authorization", value))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let message = ((chrono::Utc::now().timestamp() / 60) - 60).to_string();
    let signature = cryptfns::sign(&message, secret.as_bytes(), public.as_bytes())
        .unwrap()
        .to_bytes();

    let pubkey_as_bytes = cryptfns::mnemonic_to_bytes(pubkey.as_ref().unwrap()).unwrap();

    let value = format!(
        "Signature {} {}",
        BASE64_STANDARD.encode(&signature),
        BASE64_STANDARD.encode(&pubkey_as_bytes)
    );

    let req = test::TestRequest::post()
        .uri("/auth/self")
        .append_header(("Authorization", value))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // let body = test::read_body(resp).await;
    // println!("{:#?}", body);
}
