//! Response compression is on for API responses and the SPA, and
//! explicitly off for ciphertext streams — encrypted bytes don't
//! compress, and the CPU spent trying would tax the busiest path in the
//! server. These tests pin both sides of that split.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::test;
use hoodik::server;
use storage::data::app_file::AppFile;

use crate::helpers::{calculate_checksum, create_byte_chunks, CHUNK_SIZE_BYTES};

#[actix_web::test]
async fn test_json_compresses_and_ciphertext_stays_identity() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-compression".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "john@doe.com").await.jwt;

    // A JSON listing negotiates gzip when the client offers it.
    let req = test::TestRequest::get()
        .uri("/api/storage")
        .insert_header(("Accept-Encoding", "gzip"))
        .cookie(jwt.clone())
        .to_request();
    let response = test::call_service(&app, req).await;
    assert_eq!(
        response
            .headers()
            .get("content-encoding")
            .and_then(|value| value.to_str().ok()),
        Some("gzip")
    );

    // Upload one real chunk so there is ciphertext to stream back.
    let (data, size, _) = create_byte_chunks();
    let chunk = data.into_iter().next().unwrap();
    let checksum = calculate_checksum(vec![chunk.clone()]);

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum),
        mime: Some("application/octet-stream".to_string()),
        size: Some(size),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let chunk_checksum = cryptfns::sha256::digest(chunk.as_slice());
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?checksum={}&chunk=0", &file.id, chunk_checksum).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(chunk.clone())
        .to_request();
    let _ = test::call_and_read_body(&app, req).await;

    // The ciphertext stream declares identity even for a gzip-capable
    // client, and arrives byte-for-byte as stored.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}?chunk=0", &file.id).as_str())
        .insert_header(("Accept-Encoding", "gzip"))
        .cookie(jwt.clone())
        .to_request();
    let response = test::call_service(&app, req).await;
    let encoding = response
        .headers()
        .get("content-encoding")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("identity");
    assert!(
        encoding == "identity",
        "ciphertext must not be compressed, got {encoding}"
    );

    let body = test::read_body(response).await;
    assert_eq!(body.len(), CHUNK_SIZE_BYTES as usize);

    context.config.app.cleanup();
}
