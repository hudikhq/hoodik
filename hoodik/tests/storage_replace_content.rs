#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use hoodik::server;
use storage::data::app_file::AppFile;
use storage::data::replace_content::ReplaceContent;

use crate::helpers::create_byte_chunks;

#[actix_web::test]
async fn test_replace_content_resets_file_for_reupload() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-replace-content".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("replace@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string),
            fingerprint: Some(fingerprint),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create an editable file with hashes
    let (data, size, _) = create_byte_chunks();
    let checksum = helpers::calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("editable-note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum),
        mime: Some("text/markdown".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
        file_id: None,
        file_modified_at: None,
        md5: Some("old-md5".to_string()),
        sha1: Some("old-sha1".to_string()),
        sha256: Some("old-sha256".to_string()),
        blake2b: Some("old-blake2b".to_string()),
        cipher: None,
        editable: Some(true),
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let mut file: AppFile = serde_json::from_slice(&body).unwrap();

    // Upload all chunks to complete the file
    for (i, chunk) in data.into_iter().enumerate() {
        let chunk_checksum = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!(
            "/api/storage/{}?checksum={}&chunk={}",
            &file.id, chunk_checksum, i
        );

        let req = test::TestRequest::post()
            .uri(&uri)
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        file = serde_json::from_slice(&body).unwrap();
    }

    assert!(file.finished_upload_at.is_some());
    assert!(file.md5.is_some());

    // Replace the content
    let replace = ReplaceContent {
        size: Some(42),
        chunks: Some(1),
        cipher: None,
        encrypted_name: None,
        encrypted_thumbnail: None,
        search_tokens_hashed: Some(vec!["token1".to_string()]),
    };

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", &file.id))
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = test::read_body(resp).await;
    let updated: AppFile = serde_json::from_slice(&body).unwrap();

    // File metadata should be reset for re-upload
    assert_eq!(updated.size, Some(42));
    assert_eq!(updated.chunks, Some(1));
    assert_eq!(updated.chunks_stored, Some(0));
    assert!(updated.finished_upload_at.is_none());

    // Stale hashes must be cleared
    assert!(updated.md5.is_none());
    assert!(updated.sha1.is_none());
    assert!(updated.sha256.is_none());
    assert!(updated.blake2b.is_none());

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_replace_content_rejects_non_editable_file() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-replace-noeditable".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("noeditable@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string),
            fingerprint: Some(fingerprint),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create a non-editable file (editable = false)
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("regular-file.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("hash".to_string()),
        mime: Some("text/plain".to_string()),
        size: Some(100),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: None, // defaults to false
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let replace = ReplaceContent {
        size: Some(10),
        chunks: Some(1),
        cipher: None,
        encrypted_name: None,
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
    };

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", &file.id))
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_replace_content_rejects_directory() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-replace-dir".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("dirtest@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string),
            fingerprint: Some(fingerprint),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create a directory
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("test-dir.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("dirhash".to_string()),
        mime: Some("dir".to_string()),
        size: None,
        chunks: None,
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

    let body = test::call_and_read_body(&app, req).await;
    let dir: AppFile = serde_json::from_slice(&body).unwrap();

    let replace = ReplaceContent {
        size: Some(10),
        chunks: Some(1),
        cipher: None,
        encrypted_name: None,
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
    };

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", &dir.id))
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_replace_content_validates_size_and_chunks() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-replace-validate".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("validate@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string),
            fingerprint: Some(fingerprint),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create an editable file
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("validate-note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("valhash".to_string()),
        mime: Some("text/markdown".to_string()),
        size: Some(100),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: Some(true),
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    // size = 0 should fail validation
    let replace = ReplaceContent {
        size: Some(0),
        chunks: Some(1),
        cipher: None,
        encrypted_name: None,
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
    };

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", &file.id))
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // chunks = 0 should fail validation
    let replace = ReplaceContent {
        size: Some(10),
        chunks: Some(0),
        cipher: None,
        encrypted_name: None,
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
    };

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", &file.id))
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    context.config.app.cleanup();
}
