use actix_web::{cookie, http::StatusCode, test};
use auth::{
    auth::Auth,
    data::{
        authenticated::AuthenticatedJwt, create_user::CreateUser, credentials::Credentials,
        signature::Signature,
    },
};
use hoodik::server;

#[actix_web::test]
async fn test_registration_and_login() {
    let mut context = context::Context::mock_sqlite().await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let private_string = cryptfns::rsa::private::to_string(&private).unwrap();
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

    let nonce = Auth::generate_fingerprint_nonce(&fingerprint);
    let signature = cryptfns::rsa::private::sign(&nonce, &private_string).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/signature")
        .set_json(&Signature {
            fingerprint: Some(fingerprint.clone()),
            signature: Some(signature),
            remember: Some(false),
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let response: AuthenticatedJwt = serde_json::from_slice(&test::read_body(resp).await).unwrap();
    let jwt = format!("Bearer {}", response.jwt.clone());

    let req = test::TestRequest::post()
        .uri("/api/auth/refresh")
        .append_header(("Authorization", jwt.clone()))
        .append_header(("X-CSRF-Token", response.authenticated.session.csrf.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let response: AuthenticatedJwt = serde_json::from_slice(&test::read_body(resp).await).unwrap();
    let jwt = format!("Bearer {}", response.jwt.clone());

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .append_header(("Authorization", jwt.clone()))
        .append_header(("X-CSRF-Token", response.authenticated.session.csrf.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);

    // Doing a quick check if the cookie will be placed onto the response
    // if we are using cookies for authentication.
    context.config.use_cookies = true;

    let mut app = test::init_service(server::app(context.clone())).await;

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

    assert!(resp.headers().get("Set-Cookie").is_some());

    let response: AuthenticatedJwt = serde_json::from_slice(&test::read_body(resp).await).unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/self")
        .cookie(cookie::Cookie::new(
            context.config.get_cookie_name(),
            response.authenticated.session.token.clone(),
        ))
        .append_header(("X-CSRF-Token", response.authenticated.session.csrf.clone()))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}
