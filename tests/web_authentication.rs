use actix_web::{http::StatusCode, test};
use auth::{
    auth::Auth,
    data::{authenticated::Authenticated, create_user::CreateUser, credentials::Credentials},
};
use hoodik::server;

#[actix_web::test]
async fn test_registration_and_login() {
    let context = context::Context::mock_sqlite().await;
    let auth = Auth::new(&context);

    let private = cryptfns::rsa::private::generate().unwrap();
    let private_string = cryptfns::rsa::private::to_string(&private).unwrap();
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
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
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
        .uri("/api/auth/refresh")
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
        .uri("/api/auth/self")
        .cookie(cookie.clone())
        .append_header(("X-CSRF-Token", session.csrf.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let message = (chrono::Utc::now().timestamp() / 60).to_string();
    let signature = cryptfns::rsa::private::sign(&message, &private_string).unwrap();

    let value = format!("Signature {}", signature);

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .append_header(("Authorization", value))
        .append_header(("X-Key-Fingerprint", fingerprint.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let message = ((chrono::Utc::now().timestamp() / 60) - 60).to_string();
    let signature = cryptfns::rsa::private::sign(&message, &private_string).unwrap();

    let value = format!("Signature {}", signature);

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .append_header(("Authorization", value))
        .append_header(("X-Key-Fingerprint", fingerprint.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // let body = test::read_body(resp).await;
    // println!("{:#?}", body);
}
