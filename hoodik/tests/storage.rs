#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use auth::data::transfer_token::TransferTokenResponse;
use fs::IntoFilename;
use hoodik::server;
use storage::data::app_file::AppFile;

use crate::helpers::{calculate_checksum, create_byte_chunks, CHUNK_SIZE_BYTES, CHUNKS};

#[actix_web::test]
async fn test_creating_file_and_uploading_chunks() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let _private_string = cryptfns::rsa::private::to_string(&private).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let encrypted_secret = "some-random-encrypted-secret".to_string();

    let app = test::init_service(server::app(context.clone())).await;

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
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john2@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (second_jwt, _) = helpers::extract_cookies(resp.headers());
    let second_jwt = second_jwt.unwrap();

    let (mut data, mut size, _) = create_byte_chunks();
    assert_eq!(data.len(), size as usize / CHUNK_SIZE_BYTES as usize);

    let mut another = vec![];

    for _i in 0..(CHUNK_SIZE_BYTES / 2) {
        another.extend(b"b");
    }

    data.push(another);

    size += (CHUNK_SIZE_BYTES / 2) as i64;

    let checksum = calculate_checksum(data.clone());

    let random_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum.clone()),
        mime: Some("text/plain".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
        file_id: None,
        // Date of the file creation from the disk, if not provided we set it to now
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
        .set_json(&random_file)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    // let string_body = String::from_utf8(body.to_vec()).unwrap();
    // println!("string_body: {}", string_body);

    let mut file: AppFile = serde_json::from_slice(&body).unwrap();

    // println!("file: {:#?}", file);

    let mut uploaded = vec![];
    for (i, chunk) in data.into_iter().enumerate() {
        println!("chunk: {}", i);
        // println!("chunk: {}", i);
        let checksum = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!(
            "/api/storage/{}?checksum={}&chunk={}",
            &file.id, checksum, i
        );

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;

        file = match serde_json::from_slice(&body) {
            Ok(f) => f,
            Err(_) => {
                let string_body = String::from_utf8(body.to_vec()).unwrap();
                panic!(
                    "Failed deserializing to File struct, string_body: {}",
                    string_body
                );
            }
        };
        uploaded.push(i as i64);

        assert_eq!(file.uploaded_chunks.clone().unwrap(), uploaded);
        assert_eq!(file.chunks_stored.unwrap(), i as i64 + 1);
    }

    assert!(file.finished_upload_at.is_some());

    let filename = file.filename().unwrap();

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let contents = test::call_and_read_body(&app, req).await.to_vec();

    let content_len = contents.len();
    let file_checksum = cryptfns::sha256::digest(contents.as_slice());

    for i in 0..CHUNKS {
        let f = format!(
            "{}/{}",
            context.config.app.data_dir,
            filename.clone().with_chunk(i as i32)
        );

        println!("removing file: {}", f);
        assert!(std::fs::remove_file(f).is_ok());
    }

    assert_eq!(content_len, size as usize);
    assert_eq!(file_checksum, checksum);

    // Other user cannot see the file metadata
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(second_jwt)
        .set_json(&random_file)
        .to_request();

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Owner can see it
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&random_file)
        .to_request();

    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_transfer_token_upload_and_download() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-transfer".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    // Register a user.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("transfer@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create a file.
    let (data, size, _) = create_byte_chunks();
    let checksum = calculate_checksum(data.clone());

    let create_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("transfer-test.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum.clone()),
        mime: Some("application/octet-stream".to_string()),
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
        .set_json(&create_file)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    // ── Request an upload transfer token ──────────────────────────
    let req = test::TestRequest::post()
        .uri("/api/auth/transfer-token")
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({
            "file_id": file.id.to_string(),
            "action": "upload"
        }))
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let token_resp: TransferTokenResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(token_resp.action, "upload");
    assert_eq!(token_resp.file_id, file.id);
    let upload_token = token_resp.token;

    // ── Upload chunks using the transfer token (no cookie) ───────
    for (i, chunk) in data.iter().enumerate() {
        let checksum = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!(
            "/api/storage/{}?checksum={}&chunk={}",
            &file.id, checksum, i
        );

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .insert_header(("Authorization", format!("Bearer {}", upload_token)))
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk.clone())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Upload chunk {} failed",
            i
        );
    }

    // ── Verify upload token cannot be used for download ──────────
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}?chunk=0", &file.id).as_str())
        .insert_header(("Authorization", format!("Bearer {}", upload_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Upload token should not work for download"
    );

    // ── Verify upload token cannot be used for a different file ──
    let wrong_file_id = entity::Uuid::new_v4();
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?chunk=0", wrong_file_id).as_str())
        .insert_header(("Authorization", format!("Bearer {}", upload_token)))
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(vec![0u8; 100])
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Upload token should not work for a different file"
    );

    // ── Request a download transfer token ────────────────────────
    let req = test::TestRequest::post()
        .uri("/api/auth/transfer-token")
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({
            "file_id": file.id.to_string(),
            "action": "download"
        }))
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let token_resp: TransferTokenResponse = serde_json::from_slice(&body).unwrap();
    let download_token = token_resp.token;

    // ── Download using the transfer token (no cookie) ────────────
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}?chunk=0", &file.id).as_str())
        .insert_header(("Authorization", format!("Bearer {}", download_token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Download with transfer token failed"
    );

    // ── Update hashes using the upload transfer token ────────────
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/hashes", &file.id).as_str())
        .insert_header(("Authorization", format!("Bearer {}", upload_token)))
        .set_json(&serde_json::json!({ "sha256": checksum }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Update hashes with upload transfer token failed"
    );

    // ── Regular cookie auth still works for upload/download ──────
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}?chunk=0", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Cookie auth should still work for download"
    );

    // ── Invalid action should be rejected ────────────────────────
    let req = test::TestRequest::post()
        .uri("/api/auth/transfer-token")
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({
            "file_id": file.id.to_string(),
            "action": "delete"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Invalid action should be rejected"
    );

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_download_tar_archive() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-tar".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    // Register user.
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("tar@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create a multi-chunk file (5 × 1 MiB + 0.5 MiB = 5.5 MiB, 6 chunks).
    let (mut data, mut size, _) = create_byte_chunks();

    let mut last_chunk = vec![];
    for _ in 0..(CHUNK_SIZE_BYTES / 2) {
        last_chunk.extend(b"b");
    }
    data.push(last_chunk);
    size += (CHUNK_SIZE_BYTES / 2) as i64;

    let checksum = calculate_checksum(data.clone());
    let chunk_count = data.len();

    let create_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("tar-test.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum.clone()),
        mime: Some("application/octet-stream".to_string()),
        size: Some(size),
        chunks: Some(chunk_count as i64),
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
        .set_json(&create_file)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    // Upload all chunks.
    for (i, chunk) in data.iter().enumerate() {
        let cs = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!("/api/storage/{}?checksum={}&chunk={}", &file.id, cs, i);

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk.clone())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK, "Upload chunk {} failed", i);
    }

    // Download as tar archive.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}?format=tar", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify Content-Type and Content-Length headers.
    let content_type = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(content_type, "application/x-tar");

    let content_length: u64 = resp
        .headers()
        .get("Content-Length")
        .expect("Content-Length header should be present")
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    let tar_bytes = test::read_body(resp).await.to_vec();

    // Verify Content-Length matches actual body size.
    assert_eq!(
        tar_bytes.len() as u64,
        content_length,
        "Content-Length header must match actual tar archive size"
    );

    // Extract tar entries using the transfer crate's tar parser.
    let entries = transfer::tar::extract_tar(&tar_bytes).unwrap();

    // Verify we got the right number of chunks.
    assert_eq!(entries.len(), chunk_count);

    // Verify each entry has the correct name and matches the uploaded data.
    for (i, entry) in entries.iter().enumerate() {
        assert_eq!(
            entry.name,
            format!("{:06}.enc", i),
            "Entry {} has wrong name",
            i
        );
        assert_eq!(
            entry.data, data[i],
            "Entry {} data does not match uploaded chunk",
            i
        );
    }

    // Verify the tar archive is well-formed: expected size calculation.
    let expected_size: u64 = data
        .iter()
        .map(|chunk| {
            let chunk_len = chunk.len() as u64;
            512 + chunk_len + fs::tar::tar_padding_len(chunk_len) as u64
        })
        .sum::<u64>()
        + fs::tar::TAR_END_OF_ARCHIVE_LEN as u64;
    assert_eq!(tar_bytes.len() as u64, expected_size);

    // Verify existing download modes still work.
    // Single chunk download.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}?chunk=0", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let chunk_0 = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(chunk_0, data[0]);

    // Full concatenated download.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let full = test::call_and_read_body(&app, req).await.to_vec();
    let expected_full: Vec<u8> = data.iter().flat_map(|c| c.iter().copied()).collect();
    assert_eq!(full, expected_full);

    context.config.app.cleanup();
}
