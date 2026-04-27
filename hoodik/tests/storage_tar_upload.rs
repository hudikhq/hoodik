//! Integration tests for `POST /api/storage/{file_id}?format=tar`.
//!
//! Covers the happy path, error paths (malformed body, out-of-range chunk
//! index, duplicate index inside one archive, missing auth, wrong owner),
//! idempotent replay, partial-then-complete splits, and parity with the
//! per-chunk upload path.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use fs::tar::{tar_header, tar_padding_len, TAR_END_OF_ARCHIVE_LEN};
use fs::MAX_CHUNK_SIZE_BYTES;
use hoodik::server;
use storage::data::app_file::AppFile;

use crate::helpers::{calculate_checksum, extract_cookies};

/// Build a ustar archive directly from `(chunk_index, data)` pairs.
fn tar_of(entries: &[(i64, &[u8])]) -> Vec<u8> {
    let mut total = TAR_END_OF_ARCHIVE_LEN;
    for (_, data) in entries {
        total += 512 + data.len() + tar_padding_len(data.len() as u64);
    }
    let mut out = Vec::with_capacity(total);
    for (idx, data) in entries {
        let name = format!("{:06}.enc", idx);
        out.extend_from_slice(&tar_header(&name, data.len() as u64));
        out.extend_from_slice(data);
        out.extend(std::iter::repeat_n(0u8, tar_padding_len(data.len() as u64)));
    }
    out.extend(std::iter::repeat_n(0u8, TAR_END_OF_ARCHIVE_LEN));
    out
}

fn create_file_json(chunks: &[Vec<u8>], name: &str) -> storage::data::create_file::CreateFile {
    let size: i64 = chunks.iter().map(|c| c.len() as i64).sum();
    storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some(name.to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(calculate_checksum(chunks.to_vec())),
        mime: Some("application/octet-stream".to_string()),
        size: Some(size),
        chunks: Some(chunks.len() as i64),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: None,
    }
}

fn register_body(email: &str) -> CreateUser {
    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    CreateUser {
        email: Some(email.to_string()),
        password: Some("not-4-weak-password-for-god-sakes!".to_string()),
        secret: None,
        token: None,
        pubkey: Some(cryptfns::rsa::public::to_string(&public).unwrap()),
        fingerprint: Some(cryptfns::rsa::fingerprint(public).unwrap()),
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        invitation_id: None,
    }
}

/// Bootstrap macro: spin up the mock app, register `$email`, and bind the
/// triple `($ctx, $app, $jwt)` into the calling scope. Every test opens
/// with this so the per-test body stays focused on the scenario.
macro_rules! setup {
    ($ctx:ident, $app:ident, $jwt:ident, $data_dir:expr, $email:expr) => {
        let $ctx =
            context::Context::mock_with_data_dir(Some($data_dir.to_string())).await;
        let $app = test::init_service(server::app($ctx.clone())).await;

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(register_body($email))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let (cookie, _) = extract_cookies(resp.headers());
        let $jwt = cookie.expect("register must set session cookie");
    };
}

/// Send a tar-upload POST for `file_id` with the given request body. Used
/// by every happy-path and error-path test — encapsulates the four-line
/// request-construction boilerplate.
macro_rules! send_tar {
    ($app:expr, $jwt:expr, $file_id:expr, $tar:expr) => {{
        let req = test::TestRequest::post()
            .uri(format!("/api/storage/{}?format=tar", $file_id).as_str())
            .cookie($jwt.clone())
            .append_header(("Content-Type", "application/x-tar"))
            .set_payload($tar)
            .to_request();
        test::call_service(&$app, req).await
    }};
}

/// Create a file via `POST /api/storage` and return the deserialised
/// `AppFile`. `override_name_hash` lets a test register two files whose
/// chunk plaintexts collide on the default hash.
macro_rules! create_file {
    ($app:expr, $jwt:expr, $create:expr) => {
        create_file!($app, $jwt, $create, None::<&str>)
    };
    ($app:expr, $jwt:expr, $create:expr, $override_name_hash:expr) => {{
        let mut create = $create;
        if let Some(h) = $override_name_hash {
            create.name_hash = Some(h.to_string());
        }
        let req = test::TestRequest::post()
            .uri("/api/storage")
            .cookie($jwt.clone())
            .set_json(&create)
            .to_request();
        let body = test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<AppFile>(&body)
            .expect("create file response should be AppFile JSON")
    }};
}

/// Chunk plaintexts used by multiple tests — same-size, predictable content.
fn mock_chunks(n: usize, size: usize) -> Vec<Vec<u8>> {
    (0..n).map(|i| vec![i as u8; size]).collect()
}

/// `(chunk_index, data)` entries for every chunk, starting at 0.
fn tar_entries(chunks: &[Vec<u8>]) -> Vec<(i64, &[u8])> {
    chunks
        .iter()
        .enumerate()
        .map(|(i, c)| (i as i64, c.as_slice()))
        .collect()
}

/// Thin wrapper around a GET request — expressed as a macro because
/// actix's test-service type is awkward to name in a function signature.
macro_rules! download_bytes {
    ($app:expr, $jwt:expr, $file_id:expr) => {{
        let req = test::TestRequest::get()
            .uri(format!("/api/storage/{}", $file_id).as_str())
            .cookie($jwt.clone())
            .to_request();
        test::call_and_read_body(&$app, req).await.to_vec()
    }};
}

#[actix_web::test]
async fn test_upload_tar_happy_path() {
    setup!(context, app, jwt, "../data-test-tar-upload-happy", "tar-happy@test.com");

    let chunks = mock_chunks(4, 1024);
    let file = create_file!(app, jwt, create_file_json(&chunks, "happy.enc"));

    let resp = send_tar!(app, jwt, file.id, tar_of(&tar_entries(&chunks)));
    assert_eq!(resp.status(), StatusCode::OK);
    let body = test::read_body(resp).await;
    let updated: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(updated.chunks_stored, Some(chunks.len() as i64));
    assert!(updated.finished_upload_at.is_some());

    let expected: Vec<u8> = chunks.iter().flat_map(|c| c.iter().copied()).collect();
    assert_eq!(download_bytes!(app, jwt, file.id), expected);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&updated).await.unwrap();
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_idempotent_replay() {
    setup!(context, app, jwt, "../data-test-tar-upload-replay", "tar-replay@test.com");

    let chunks = mock_chunks(3, 512);
    let file = create_file!(app, jwt, create_file_json(&chunks, "replay.enc"));
    let tar = tar_of(&tar_entries(&chunks));

    // Second replay with identical bytes must succeed — tar intentionally
    // overwrites, unlike the per-chunk endpoint which rejects duplicates.
    for _ in 0..2 {
        let resp = send_tar!(app, jwt, file.id, tar.clone());
        assert_eq!(resp.status(), StatusCode::OK);
    }

    let expected: Vec<u8> = chunks.iter().flat_map(|c| c.iter().copied()).collect();
    assert_eq!(download_bytes!(app, jwt, file.id), expected);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_partial_then_complete() {
    setup!(context, app, jwt, "../data-test-tar-upload-split", "tar-split@test.com");

    let chunks = mock_chunks(5, 256);
    let file = create_file!(app, jwt, create_file_json(&chunks, "split.enc"));

    let first: Vec<(i64, &[u8])> = chunks[..2]
        .iter()
        .enumerate()
        .map(|(i, c)| (i as i64, c.as_slice()))
        .collect();
    let resp = send_tar!(app, jwt, file.id, tar_of(&first));
    assert_eq!(resp.status(), StatusCode::OK);
    let mid: AppFile = serde_json::from_slice(&test::read_body(resp).await).unwrap();
    assert_eq!(mid.chunks_stored, Some(2));
    assert!(mid.finished_upload_at.is_none(), "still in progress");

    let second: Vec<(i64, &[u8])> = chunks[2..]
        .iter()
        .enumerate()
        .map(|(i, c)| (i as i64 + 2, c.as_slice()))
        .collect();
    let resp = send_tar!(app, jwt, file.id, tar_of(&second));
    assert_eq!(resp.status(), StatusCode::OK);
    let done: AppFile = serde_json::from_slice(&test::read_body(resp).await).unwrap();
    assert_eq!(done.chunks_stored, Some(5));
    assert!(done.finished_upload_at.is_some());

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&done).await.unwrap();
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_malformed_rejected() {
    setup!(context, app, jwt, "../data-test-tar-upload-malformed", "tar-malformed@test.com");

    let chunks = mock_chunks(1, 128);
    let file = create_file!(app, jwt, create_file_json(&chunks, "malformed.enc"));

    // 2 KiB of non-header bytes — past the "need more data" guard, into a
    // real header-parse attempt.
    let resp = send_tar!(app, jwt, file.id, vec![0x7Fu8; 2048]);
    assert!(resp.status().is_client_error(), "got {}", resp.status());

    use fs::prelude::{Fs, FsProviderContract};
    assert!(
        Fs::new(&context.config)
            .get_uploaded_chunks(&file)
            .await
            .unwrap()
            .is_empty(),
        "malformed request must not leave chunks on disk"
    );
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_out_of_range_chunk_rejected() {
    setup!(context, app, jwt, "../data-test-tar-upload-oor", "tar-oor@test.com");

    let chunks = mock_chunks(2, 64);
    let file = create_file!(app, jwt, create_file_json(&chunks, "oor.enc"));

    // Index 2 is past the end for a 2-chunk file.
    let resp = send_tar!(app, jwt, file.id, tar_of(&[(2, b"illegal")]));
    assert!(resp.status().is_client_error(), "got {}", resp.status());
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_duplicate_index_rejected() {
    setup!(context, app, jwt, "../data-test-tar-upload-dup", "tar-dup@test.com");

    let chunks = mock_chunks(3, 32);
    let file = create_file!(app, jwt, create_file_json(&chunks, "dup.enc"));

    let resp = send_tar!(app, jwt, file.id, tar_of(&[(0, b"first"), (0, b"second")]));
    assert!(resp.status().is_client_error(), "got {}", resp.status());
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_unauthenticated() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data-test-tar-upload-unauth".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?format=tar", entity::Uuid::new_v4()).as_str())
        .append_header(("Content-Type", "application/x-tar"))
        .set_payload(tar_of(&[(0, b"irrelevant")]))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_not_owned() {
    setup!(context, app, owner_jwt, "../data-test-tar-upload-notowned", "tar-owner@test.com");

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(register_body("tar-stranger@test.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (stranger_jwt, _) = extract_cookies(resp.headers());
    let stranger_jwt = stranger_jwt.unwrap();

    let chunks = mock_chunks(2, 32);
    let file = create_file!(app, owner_jwt, create_file_json(&chunks, "private.enc"));

    // Either 403 or 404 is acceptable — the server can choose not to leak
    // existence to unrelated users. Both are 4xx client errors.
    let resp = send_tar!(app, stranger_jwt, file.id, tar_of(&[(0, &chunks[0])]));
    assert!(resp.status().is_client_error(), "got {}", resp.status());
    context.config.app.cleanup();
}

/// End-to-end reproduction of the Flutter upload failure observed on
/// 2026-04-27: any file ≥ 4 MiB bounces off the tar route with
/// `chunk_size_exceeds_max`. Encrypts a 12 MiB payload through real
/// AEGIS-128L exactly the way every Hoodik client does, packs the three
/// 4 MiB-plus-16 B ciphertexts into the same tar shape `transfer::upload_tar`
/// emits, and POSTs it. The existing tar tests use synthetic chunks of a
/// few KiB so they never crossed the encrypted-size ceiling that real
/// uploads sit on.
#[actix_web::test]
async fn test_upload_tar_three_chunk_aegis_encrypted_file() {
    setup!(
        context,
        app,
        jwt,
        "../data-test-tar-upload-multi-chunk-aegis",
        "tar-multi-aegis@test.com"
    );

    let chunk_count = 3usize;
    let plaintext_chunk_len = MAX_CHUNK_SIZE_BYTES as usize;
    let key = cryptfns::aegis::generate_key().unwrap();

    let encrypted_chunks: Vec<Vec<u8>> = (0..chunk_count)
        .map(|i| {
            let plaintext: Vec<u8> = (0..plaintext_chunk_len)
                .map(|n| ((i * 17 + n) as u8).wrapping_mul(31))
                .collect();
            cryptfns::aegis::encrypt(key.clone(), plaintext).unwrap()
        })
        .collect();

    for (i, c) in encrypted_chunks.iter().enumerate() {
        assert_eq!(
            c.len(),
            plaintext_chunk_len + 16,
            "chunk {i} must carry the AEGIS-128L 16-byte tag",
        );
    }

    let file = create_file!(
        app,
        jwt,
        create_file_json(&encrypted_chunks, "multi-chunk-aegis.enc")
    );

    let resp = send_tar!(app, jwt, file.id, tar_of(&tar_entries(&encrypted_chunks)));
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "tar upload of a 3-chunk AEGIS-encrypted file must succeed; this is \
         the exact shape the Flutter app produces for any file ≥ 4 MiB",
    );

    let updated: AppFile = serde_json::from_slice(&test::read_body(resp).await).unwrap();
    assert_eq!(updated.chunks_stored, Some(chunk_count as i64));
    assert!(updated.finished_upload_at.is_some());

    let expected: Vec<u8> = encrypted_chunks
        .iter()
        .flat_map(|c| c.iter().copied())
        .collect();
    assert_eq!(download_bytes!(app, jwt, file.id), expected);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&updated).await.unwrap();
    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_upload_tar_matches_per_chunk_final_state() {
    setup!(context, app, jwt, "../data-test-tar-upload-parity", "tar-parity@test.com");

    let chunks = mock_chunks(4, 2048);
    let file_tar = create_file!(
        app,
        jwt,
        create_file_json(&chunks, "parity-tar.enc"),
        Some("tar-name-hash")
    );
    let file_pc = create_file!(
        app,
        jwt,
        create_file_json(&chunks, "parity-pc.enc"),
        Some("pc-name-hash")
    );

    let resp = send_tar!(app, jwt, file_tar.id, tar_of(&tar_entries(&chunks)));
    assert_eq!(resp.status(), StatusCode::OK);

    for (i, chunk) in chunks.iter().enumerate() {
        let cs = cryptfns::sha256::digest(chunk.as_slice());
        let req = test::TestRequest::post()
            .uri(format!("/api/storage/{}?checksum={}&chunk={}", &file_pc.id, cs, i).as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK, "per-chunk upload {}", i);
    }

    assert_eq!(
        download_bytes!(app, jwt, file_tar.id),
        download_bytes!(app, jwt, file_pc.id),
        "tar and per-chunk uploads must produce byte-identical files"
    );

    use fs::prelude::{Fs, FsProviderContract};
    let fs = Fs::new(&context.config);
    fs.purge_all(&file_tar).await.unwrap();
    fs.purge_all(&file_pc).await.unwrap();
    context.config.app.cleanup();
}
