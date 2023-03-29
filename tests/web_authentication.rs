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

    let pubkey = cryptfns::get_pubkey_as_mnemonic();

    let mut app = test::init_service(server::app(context.clone())).await;
    let req = test::TestRequest::post()
        .uri("/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey,
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
        .manage_cookie(&authenticated.session, false)
        .await
        .unwrap();

    let req = test::TestRequest::post()
        .uri("/auth/refresh")
        .cookie(cookie)
        .append_header(("X-CSRF-Token", authenticated.session.csrf))
        .to_request();

    let resp = test::call_service(&mut app, req).await;

    assert_eq!(resp.status(), StatusCode::OK);
}
