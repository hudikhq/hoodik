use actix_web::{http, test};
use hoodik::server;

#[actix_web::test]
async fn cors_preflight_allows_patch_requests() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context)).await;

    let req = test::TestRequest::default()
        .method(http::Method::OPTIONS)
        .uri("/api/users/me")
        .insert_header((http::header::ORIGIN, "https://app.example.test"))
        .insert_header((http::header::ACCESS_CONTROL_REQUEST_METHOD, "PATCH"))
        .insert_header((http::header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    let methods = resp
        .headers()
        .get(http::header::ACCESS_CONTROL_ALLOW_METHODS)
        .expect("preflight response should include allowed methods")
        .to_str()
        .expect("allowed methods should be valid header text");
    assert!(methods.contains("PATCH"));
}
