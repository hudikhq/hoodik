#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use hoodik::server;
use storage::data::app_file::AppFile;
use storage::data::set_editable::SetEditable;

use crate::helpers::create_byte_chunks;

#[actix_web::test]
async fn test_set_editable_converts_file_to_note() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-set-editable-1".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("editable-1@test.com".to_string()),
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

    // Create a non-editable markdown file
    let (data, size, _) = create_byte_chunks();
    let checksum = helpers::calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum),
        mime: Some("text/markdown".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: None, // not editable
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(!file.editable);

    // Convert to editable note
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(jwt.clone())
        .set_json(&SetEditable {
            editable: Some(true),
        })
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let updated: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(updated.editable);
    assert_eq!(updated.id, file.id);

    // Verify via metadata endpoint
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", file.id))
        .cookie(jwt.clone())
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let fetched: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(fetched.editable);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_set_editable_can_revert_note_to_regular_file() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-set-editable-2".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("editable-2@test.com".to_string()),
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
    let (data, size, _) = create_byte_chunks();
    let checksum = helpers::calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum),
        mime: Some("text/markdown".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
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
    assert!(file.editable);

    // Revert to non-editable
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(jwt.clone())
        .set_json(&SetEditable {
            editable: Some(false),
        })
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let updated: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(!updated.editable);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_set_editable_rejects_directory() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-set-editable-3".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("editable-3@test.com".to_string()),
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
        encrypted_name: Some("dir.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("dir-hash-editable".to_string()),
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

    // Try to set editable on a directory — should fail
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", dir.id))
        .cookie(jwt.clone())
        .set_json(&SetEditable {
            editable: Some(true),
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_set_editable_requires_editable_field() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-set-editable-4".to_string()))
            .await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("editable-4@test.com".to_string()),
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

    // Create a file
    let (data, size, _) = create_byte_chunks();
    let checksum = helpers::calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum),
        mime: Some("text/markdown".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
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
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    // Send with editable: None — validation should reject
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(jwt.clone())
        .set_json(&SetEditable { editable: None })
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    context.config.app.cleanup();
}
