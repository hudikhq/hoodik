#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use hoodik::server;
use storage::data::app_file::AppFile;

/// `replaceContent` on a non-editable file is rejected with 400. The flag
/// is opt-in — callers have to set `editable = true` at create time (or
/// flip it later via `setEditable`) before the atomic-edit flow is allowed.
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
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({ "size": 10, "chunks": 1 }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

/// Directories are not editable content. `replaceContent` on a directory
/// is rejected with 400 before any pending-version bookkeeping happens.
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

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", dir.id).as_str())
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({ "size": 10, "chunks": 1 }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

/// `size` and `chunks` must be positive. Validation runs before any
/// repository work, so a zero on either field surfaces as 422 with no
/// side effects on the file row.
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

    for payload in [
        serde_json::json!({ "size": 0, "chunks": 1 }),
        serde_json::json!({ "size": 10, "chunks": 0 }),
    ] {
        let req = test::TestRequest::put()
            .uri(format!("/api/storage/{}/content", file.id).as_str())
            .cookie(jwt.clone())
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    context.config.app.cleanup();
}
