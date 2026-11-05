#[path = "./helpers.rs"]
mod helpers;

use actix_web::{cookie::Cookie, http::StatusCode, test};
use hoodik::server;
use storage::data::app_file::AppFile;
use storage::data::favorite::{FavoriteMany, SetFavorite};
use storage::data::response::Response;

async fn create_file(app: &impl helpers::TestApp, jwt: &Cookie<'static>, seed: &str) -> AppFile {
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some(format!("{seed}.enc")),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(helpers::name_tag(seed)),
        mime: Some("text/plain".to_string()),
        size: Some(1),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        digest_tokens_root: None,
        digest_tokens_file: None,
        cipher: None,
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(app, req).await;
    serde_json::from_slice(&body).unwrap()
}

#[actix_web::test]
async fn test_set_favorite_toggles_single_file() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-favorite-1".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "favorite-1@test.com")
        .await
        .jwt;

    let file = create_file(&app, &jwt, "favorite-one").await;
    assert!(!file.favorite, "new files start unfavourited");

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/favorite", file.id))
        .cookie(jwt.clone())
        .set_json(&SetFavorite {
            favorite: Some(true),
        })
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let updated: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(updated.favorite);
    assert_eq!(updated.id, file.id);

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", file.id))
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let fetched: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(fetched.favorite);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/favorite", file.id))
        .cookie(jwt.clone())
        .set_json(&SetFavorite {
            favorite: Some(false),
        })
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let reverted: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(!reverted.favorite);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_favorite_many_updates_batch_and_listing_filter() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-favorite-2".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "favorite-2@test.com")
        .await
        .jwt;

    let one = create_file(&app, &jwt, "favorite-batch-one").await;
    let two = create_file(&app, &jwt, "favorite-batch-two").await;

    let req = test::TestRequest::post()
        .uri("/api/storage/favorite-many")
        .cookie(jwt.clone())
        .set_json(&FavoriteMany {
            ids: Some(vec![one.id, two.id]),
            favorite: Some(true),
        })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = test::TestRequest::get()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let all: Response = serde_json::from_slice(&body).unwrap();
    assert!(all.children.iter().all(|f| f.favorite));

    let req = test::TestRequest::get()
        .uri("/api/storage?favorite=true")
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let favorites: Response = serde_json::from_slice(&body).unwrap();
    let mut ids: Vec<_> = favorites.children.iter().map(|f| f.id).collect();
    ids.sort();
    let mut expected = vec![one.id, two.id];
    expected.sort();
    assert_eq!(ids, expected);

    let req = test::TestRequest::post()
        .uri("/api/storage/favorite-many")
        .cookie(jwt.clone())
        .set_json(&FavoriteMany {
            ids: Some(vec![one.id, two.id]),
            favorite: Some(false),
        })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = test::TestRequest::get()
        .uri("/api/storage?favorite=true")
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let favorites: Response = serde_json::from_slice(&body).unwrap();
    assert!(favorites.children.is_empty());

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_favorite_rejects_unknown_file() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-favorite-3".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "favorite-3@test.com")
        .await
        .jwt;

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/favorite", entity::Uuid::new_v4()))
        .cookie(jwt.clone())
        .set_json(&SetFavorite {
            favorite: Some(true),
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_favorite_many_requires_ids() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-favorite-4".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "favorite-4@test.com")
        .await
        .jwt;

    let req = test::TestRequest::post()
        .uri("/api/storage/favorite-many")
        .cookie(jwt.clone())
        .set_json(&FavoriteMany {
            ids: Some(vec![]),
            favorite: Some(true),
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    context.config.app.cleanup();
}
