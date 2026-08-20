//! Integration tests for the direct-transfer manifest routes.
//!
//! These run against the default local-filesystem provider, which is exactly
//! the point: the routes have to exist, authenticate, and then refuse — a
//! deployment with nothing to hand out must say so plainly rather than
//! producing a manifest the client cannot use. The S3 side of the same
//! surface is covered by the presign suite in the `fs` crate, which needs a
//! real bucket.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use hoodik::server;
use storage::data::app_file::AppFile;

use crate::helpers::{calculate_checksum, create_byte_chunks};

/// Create a file record and upload every chunk, returning the finished file.
async fn uploaded_file(
    app: &impl helpers::TestApp,
    jwt: &actix_web::cookie::Cookie<'static>,
) -> AppFile {
    let (data, size, _) = create_byte_chunks();
    let checksum = calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        name_hash: Some(helpers::name_tag(&checksum)),
        mime: Some("text/plain".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
        file_id: None,
        file_modified_at: None,
        md5: Some("asd".to_string()),
        sha1: Some("asd".to_string()),
        sha256: Some("asd".to_string()),
        blake2b: Some("asd".to_string()),
        cipher: None,
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();
    let body = test::call_and_read_body(app, req).await;
    let mut file: AppFile = serde_json::from_slice(&body).unwrap();

    for (i, chunk) in data.into_iter().enumerate() {
        let checksum = cryptfns::sha256::digest(chunk.as_slice());
        let req = test::TestRequest::post()
            .uri(&format!(
                "/api/storage/{}?checksum={}&chunk={}",
                &file.id, checksum, i
            ))
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk)
            .to_request();
        let body = test::call_and_read_body(app, req).await;
        file = serde_json::from_slice(&body).unwrap();
    }

    file
}

/// An unauthenticated caller gets nothing. The manifest is the only gate on
/// the direct path, so it has to be at least as strict as the byte route it
/// stands in for.
#[actix_web::test]
async fn download_urls_rejects_an_anonymous_caller() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-anon@doe.com").await.jwt;
    let file = uploaded_file(&app, &jwt).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/chunk-urls", file.id))
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// Someone else's file is not found, the same answer the download route
/// gives — a manifest must not become a way to probe for file ids.
#[actix_web::test]
async fn download_urls_hides_another_users_file() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::register_curve25519(&app, "cu-owner@doe.com")
        .await
        .jwt;
    let stranger = helpers::register_curve25519(&app, "cu-stranger@doe.com")
        .await
        .jwt;

    let file = uploaded_file(&app, &owner).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/chunk-urls", file.id))
        .cookie(stranger)
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Same answer when the owner has just been served: the metadata cache is
/// keyed by caller, so one user's lookup must never satisfy another's. This
/// is the sequence that used to hand a second user the owner's row —
/// existence, size, content hashes and working URLs included.
#[actix_web::test]
async fn download_urls_hide_a_file_the_owner_just_fetched() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::register_curve25519(&app, "cu-warm-owner@doe.com")
        .await
        .jwt;
    let stranger = helpers::register_curve25519(&app, "cu-warm-stranger@doe.com")
        .await
        .jwt;

    let file = uploaded_file(&app, &owner).await;

    // Warm the cache as the owner. Local storage then refuses for lack of
    // URLs, which is fine — the lookup has already run and been memoized.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/chunk-urls", file.id))
        .cookie(owner)
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/chunk-urls", file.id))
        .cookie(stranger)
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// On local storage there is no URL to give. The route says so rather than
/// returning an empty manifest that a client would read as "no chunks".
#[actix_web::test]
async fn download_urls_refuses_when_the_provider_has_no_urls() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-local@doe.com").await.jwt;
    let file = uploaded_file(&app, &jwt).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/chunk-urls", file.id))
        .cookie(jwt)
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = test::read_body(response).await;
    let text = String::from_utf8_lossy(&body);
    assert!(
        text.contains("direct_transfer_unavailable"),
        "expected a clear reason, got: {text}"
    );
}

/// Upload URLs are refused the same way, and the refusal comes after the
/// write permission check rather than instead of it.
#[actix_web::test]
async fn upload_urls_refuses_when_the_provider_has_no_urls() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-up@doe.com").await.jwt;
    let file = uploaded_file(&app, &jwt).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/storage/{}/upload-urls", file.id))
        .cookie(jwt)
        .set_json(serde_json::json!({ "chunks": [{ "chunk": 0, "size": 1024 }] }))
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// A chunk index outside the file's range is rejected before any URL is
/// signed. Without this a client could ask for, and be handed, a signed write
/// to a key the file does not own.
#[actix_web::test]
async fn upload_urls_rejects_an_out_of_range_chunk() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-range@doe.com").await.jwt;
    let file = uploaded_file(&app, &jwt).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/storage/{}/upload-urls", file.id))
        .cookie(jwt)
        .set_json(serde_json::json!({
            "chunks": [{ "chunk": 9_999, "size": 1024 }]
        }))
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// A body larger than one chunk is refused at manifest time. This is the only
/// place the size limit can be applied on the direct path, because after the
/// URL is signed nothing of ours sits in front of the write.
#[actix_web::test]
async fn upload_urls_rejects_an_oversized_chunk() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-big@doe.com").await.jwt;
    let file = uploaded_file(&app, &jwt).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/storage/{}/upload-urls", file.id))
        .cookie(jwt)
        .set_json(serde_json::json!({
            "chunks": [{ "chunk": 0, "size": fs::MAX_CHUNK_PAYLOAD_BYTES + 1 }]
        }))
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// The same chunk twice in one request is a client bug, and signing both would
/// hand out two writes to one key.
#[actix_web::test]
async fn upload_urls_rejects_a_duplicate_chunk_index() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-dup@doe.com").await.jwt;
    let file = uploaded_file(&app, &jwt).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/storage/{}/upload-urls", file.id))
        .cookie(jwt)
        .set_json(serde_json::json!({
            "chunks": [{ "chunk": 0, "size": 16 }, { "chunk": 0, "size": 16 }]
        }))
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// Nothing reports back when a client writes into the bucket, so finalize
/// asks the store what actually landed. A file whose chunks are not all there
/// must not have its version pointer moved.
#[actix_web::test]
async fn finalize_refuses_a_file_with_chunks_missing() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "cu-fin@doe.com").await.jwt;

    // Declared with chunks, none uploaded.
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("unfinished".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        name_hash: Some("unfinished-hash".to_string()),
        mime: Some("text/plain".to_string()),
        size: Some(4096),
        chunks: Some(3),
        file_id: None,
        file_modified_at: None,
        md5: Some("asd".to_string()),
        sha1: Some("asd".to_string()),
        sha256: Some("asd".to_string()),
        blake2b: Some("asd".to_string()),
        cipher: None,
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/storage/{}/finalize", file.id))
        .cookie(jwt)
        .to_request();
    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

/// A local-storage deployment advertises the capability as off, which is what
/// keeps every client on the relaying path.
#[actix_web::test]
async fn capabilities_report_direct_transfer_off_on_local_storage() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::get().uri("/api/capabilities").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let caps: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(caps["direct_transfer"], serde_json::Value::Bool(false));
}
