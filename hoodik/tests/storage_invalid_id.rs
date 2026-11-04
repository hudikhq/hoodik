//! A malformed id in the path is the caller's mistake, not a server fault.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use hoodik::server;

#[actix_web::test]
async fn metadata_with_a_malformed_id_is_a_bad_request() {
    let context = context::Context::mock_with_data_dir(Some("../data/test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "invalid-id@doe.com")
        .await
        .jwt;

    for id in ["not-a-uuid", "__shared_with_me__"] {
        let req = test::TestRequest::get()
            .uri(&format!("/api/storage/{id}/metadata"))
            .cookie(jwt.clone())
            .to_request();
        let response = test::call_service(&app, req).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "id: {id}");
    }
}
