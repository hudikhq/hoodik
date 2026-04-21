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

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let contents = test::call_and_read_body(&app, req).await.to_vec();

    let content_len = contents.len();
    let file_checksum = cryptfns::sha256::digest(contents.as_slice());

    // Chunks now live under `{data_dir}/{uuid}/v{version}/` instead of the
    // legacy flat `{data_dir}/{timestamp}-{uuid}.part.{n}` layout. Use
    // `purge_all` so this test stays correct regardless of which layout
    // the FS provider used for this file.
    use fs::prelude::{Fs, FsProviderContract};
    let fs = Fs::new(&context.config);
    fs.purge_all(&file).await.unwrap();

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

/// End-to-end exercise of the versioned-chunks atomicity contract:
/// a file is created and uploaded, then edited (replaceContent + new
/// chunks). The active version must flip from 1 → 2, the previous
/// version must be preserved in `file_versions`, and the download must
/// reflect the new content.
#[actix_web::test]
async fn test_replace_content_atomic_edit() {
    use entity::{ColumnTrait, EntityTrait, QueryFilter};

    let context =
        context::Context::mock_with_data_dir(Some("../data-test-versioning".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("editor@test.com".to_string()),
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

    // ── Create the editable file with v1 content ────────────────
    let v1_data = vec![b"version-one-content".to_vec()];
    let v1_size = v1_data[0].len() as i64;
    let v1_checksum = calculate_checksum(v1_data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(v1_checksum.clone()),
        mime: Some("text/markdown".to_string()),
        size: Some(v1_size),
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
    assert_eq!(file.active_version, 1);
    assert!(file.pending_version.is_none());

    // Upload the v1 chunk and finalize.
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?chunk=0&checksum={}",
            file.id,
            cryptfns::sha256::digest(v1_data[0].as_slice())).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v1_data[0].clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(file.finished_upload_at.is_some());
    assert_eq!(file.active_version, 1);
    assert!(file.pending_version.is_none(), "first commit leaves no pending");

    // ── Edit: replaceContent with v2 metadata ────────────────────
    let v2_data = vec![b"version-two-totally-different-content!".to_vec()];
    let v2_size = v2_data[0].len() as i64;

    let replace = serde_json::json!({
        "size": v2_size,
        "chunks": 1,
    });
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.active_version, 1, "active stays at v1 mid-edit");
    assert_eq!(file.pending_version, Some(2));
    assert_eq!(file.pending_chunks, Some(1));
    assert_eq!(file.pending_size, Some(v2_size));
    assert_eq!(file.size, Some(v1_size), "size still describes the active v1");

    // Download mid-edit must still serve v1 content unchanged.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let body_mid = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(body_mid, v1_data[0], "mid-edit reads see the previous version");

    // Upload the v2 chunk → triggers auto-finalize (pointer swap).
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?chunk=0&checksum={}",
            file.id,
            cryptfns::sha256::digest(v2_data[0].as_slice())).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v2_data[0].clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.active_version, 2, "active flips to v2 after finalize");
    assert!(file.pending_version.is_none(), "pending cleared after finalize");
    assert_eq!(file.size, Some(v2_size), "size now describes v2");

    // Download after edit must serve v2.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let body_after = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(body_after, v2_data[0], "post-edit reads see v2 content");

    // History row for v=1 must exist.
    let history = entity::file_versions::Entity::find()
        .filter(entity::file_versions::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(history.len(), 1, "v=1 snapshotted into file_versions");
    assert_eq!(history[0].version, 1);
    assert_eq!(history[0].chunks, 1);
    assert_eq!(history[0].size, v1_size);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// A second `replaceContent` while a pending edit is still in progress
/// must return 409 — that's the safety net against accidental concurrent
/// edits from a second device.
#[actix_web::test]
async fn test_replace_content_concurrent_returns_409() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-409".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("conflict@test.com".to_string()),
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

    let data = b"initial-content".to_vec();
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(cryptfns::sha256::digest(data.as_slice())),
        mime: Some("text/markdown".to_string()),
        size: Some(data.len() as i64),
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

    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?chunk=0&checksum={}",
            file.id,
            cryptfns::sha256::digest(data.as_slice())).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(data.clone())
        .to_request();
    let _ = test::call_and_read_body(&app, req).await;

    // First replaceContent — sets pending_version, no chunks uploaded yet.
    let replace = serde_json::json!({ "size": 100, "chunks": 5 });
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Second replaceContent without `force` — must 409.
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&replace)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "concurrent replaceContent without force must 409"
    );

    // Third replaceContent with `force = true` — must succeed; allocates
    // a fresh pending_version above the abandoned one.
    let force_replace = serde_json::json!({ "size": 200, "chunks": 3, "force": true });
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&force_replace)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(
        file.pending_version.unwrap() >= 3,
        "force-replace allocates above the abandoned pending"
    );
    assert_eq!(file.pending_chunks, Some(3));
    assert_eq!(file.pending_size, Some(200));

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// Three saves of an editable file → versions list shows the two
/// historical snapshots → restoring v1 puts that content back as the
/// active version while preserving the prior active in history → the
/// historical row count still reflects every prior state.
#[actix_web::test]
async fn test_versions_list_and_restore() {
    use entity::{ColumnTrait, EntityTrait, QueryFilter};

    let context =
        context::Context::mock_with_data_dir(Some("../data-test-versions-restore".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("history@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string),
            fingerprint: Some(fingerprint),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();
    let (jwt, _) = helpers::extract_cookies(test::call_service(&app, req).await.headers());
    let jwt = jwt.unwrap();

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("name-hash".to_string()),
        mime: Some("text/markdown".to_string()),
        size: Some(3),
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
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let upload_chunk = |jwt: actix_web::cookie::Cookie<'static>, fid: entity::Uuid, data: Vec<u8>| {
        let app = &app;
        async move {
            let cs = cryptfns::sha256::digest(data.as_slice());
            let req = test::TestRequest::post()
                .uri(format!("/api/storage/{}?chunk=0&checksum={}", fid, cs).as_str())
                .cookie(jwt)
                .append_header(("Content-Type", "application/octet-stream"))
                .set_payload(data)
                .to_request();
            let body = test::call_and_read_body(app, req).await;
            serde_json::from_slice::<AppFile>(&body).unwrap()
        }
    };

    let v1_bytes = b"AAA".to_vec();
    let v2_bytes = b"BBB".to_vec();
    let v3_bytes = b"CCC".to_vec();

    let _ = upload_chunk(jwt.clone(), file.id, v1_bytes.clone()).await;

    // Edit twice — produces history rows for v1 and v2.
    for content in [&v2_bytes, &v3_bytes] {
        let req = test::TestRequest::put()
            .uri(format!("/api/storage/{}/content", file.id).as_str())
            .cookie(jwt.clone())
            .set_json(&serde_json::json!({ "size": content.len(), "chunks": 1 }))
            .to_request();
        let _ = test::call_and_read_body(&app, req).await;
        let _ = upload_chunk(jwt.clone(), file.id, content.clone()).await;
    }

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/versions", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let versions: Vec<entity::file_versions::Model> =
        serde_json::from_slice(&body).unwrap();
    let history_versions: Vec<i32> = versions.iter().map(|v| v.version).collect();
    assert_eq!(
        history_versions,
        vec![2, 1],
        "history is ordered newest-first; active (v3) is not listed"
    );

    // Restore v1 — content reverts and the previously-active v3 is
    // captured as a new history row.
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}/versions/1/restore", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let restored: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(
        restored.active_version > 3,
        "restore allocates a fresh slot above any existing version"
    );

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let downloaded = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(downloaded, v1_bytes, "restored content matches the v1 source");

    let history_count = entity::file_versions::Entity::find()
        .filter(entity::file_versions::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap()
        .len();
    assert_eq!(
        history_count, 3,
        "v1 + v2 + v3 in history; the active restored copy lives on the file row"
    );

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&restored).await.unwrap();
    context.config.app.cleanup();
}

/// Forking a historical version creates a new file whose content
/// matches that snapshot byte-for-byte. The original file's active
/// version stays untouched.
#[actix_web::test]
async fn test_fork_creates_independent_copy() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-fork".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("fork@test.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string),
            fingerprint: Some(fingerprint),
            encrypted_private_key: Some("encrypted-secret".to_string()),
            invitation_id: None,
        })
        .to_request();
    let (jwt, _) = helpers::extract_cookies(test::call_service(&app, req).await.headers());
    let jwt = jwt.unwrap();

    // Source: a 1-chunk editable file with v1 content, then edited to v2.
    let v1_bytes = b"original-snapshot".to_vec();
    let v2_bytes = b"current-content---".to_vec();

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("source-key".to_string()),
        encrypted_name: Some("source.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some("source-hash".to_string()),
        mime: Some("text/markdown".to_string()),
        size: Some(v1_bytes.len() as i64),
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
    let source: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let req = test::TestRequest::post()
        .uri(format!(
            "/api/storage/{}?chunk=0&checksum={}",
            source.id,
            cryptfns::sha256::digest(v1_bytes.as_slice())
        ).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v1_bytes.clone())
        .to_request();
    let _ = test::call_and_read_body(&app, req).await;

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", source.id).as_str())
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({ "size": v2_bytes.len(), "chunks": 1 }))
        .to_request();
    let _ = test::call_and_read_body(&app, req).await;

    let req = test::TestRequest::post()
        .uri(format!(
            "/api/storage/{}?chunk=0&checksum={}",
            source.id,
            cryptfns::sha256::digest(v2_bytes.as_slice())
        ).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v2_bytes.clone())
        .to_request();
    let _ = test::call_and_read_body(&app, req).await;

    // Fork v1 into a new note. The chunks/size in the body are ignored
    // — server takes them from the source version's recorded values.
    let new_note = serde_json::json!({
        "encrypted_key": "fork-key",
        "encrypted_name": "source (restored).md",
        "name_hash": "fork-hash",
        "mime": "text/markdown",
        "size": 999,
        "chunks": 999,
        "editable": true,
    });
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}/versions/1/fork", source.id).as_str())
        .cookie(jwt.clone())
        .set_json(&new_note)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let forked: AppFile = serde_json::from_slice(&body).unwrap();
    assert_ne!(forked.id, source.id);
    assert_eq!(forked.size, Some(v1_bytes.len() as i64), "fork uses source v1 size");
    assert_eq!(forked.active_version, 1);
    assert!(forked.finished_upload_at.is_some());

    // Forked file's content matches v1, source still serves v2.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", forked.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let forked_content = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(forked_content, v1_bytes);

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", source.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let source_content = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(source_content, v2_bytes, "source untouched by fork");

    use fs::prelude::{Fs, FsProviderContract};
    let fs = Fs::new(&context.config);
    fs.purge_all(&source).await.unwrap();
    fs.purge_all(&forked).await.unwrap();
    context.config.app.cleanup();
}
