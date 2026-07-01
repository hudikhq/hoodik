//! Instance-wide storage quota (`STORAGE_INSTANCE_QUOTA_BYTES`) enforcement.
//!
//! The per-user quota caps one account; the instance ceiling caps the whole
//! deployment regardless of how many accounts share it. These tests drive the
//! three write entry points (create, tar pre-read, tar mid-stream) plus the
//! concurrent-create race the process-level reserve lock exists to close.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use entity::{files, EntityTrait};
use fs::tar::{tar_header, tar_padding_len, TAR_END_OF_ARCHIVE_LEN};
use hoodik::server;
use serde_json::Value;
use storage::data::app_file::AppFile;
use storage::data::create_file::CreateFile;

use crate::helpers::extract_cookies;

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

/// A `CreateFile` whose declared size is the sum of `chunk_lens`. Quota is
/// reserved against this declared size at creation, so the chunk shape decides
/// how much instance budget the file consumes. The encrypted key is opaque to
/// the server, so a placeholder is enough for the quota path.
fn sized_create(name_hash: &str, chunk_lens: &[usize]) -> CreateFile {
    CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("encrypted-name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(name_hash.to_string()),
        mime: Some("application/octet-stream".to_string()),
        size: Some(chunk_lens.iter().map(|n| *n as i64).sum()),
        chunks: Some(chunk_lens.len() as i64),
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

/// Register `$email` against `$app` and bind the session cookie to `$jwt`.
macro_rules! register {
    ($app:expr, $jwt:ident, $email:expr) => {
        let $jwt = {
            let req = test::TestRequest::post()
                .uri("/api/auth/register")
                .set_json(register_body($email))
                .to_request();
            let resp = test::call_service(&$app, req).await;
            assert!(resp.status().is_success(), "register failed: {:?}", resp.status());
            let (jwt, _) = extract_cookies(resp.headers());
            jwt.expect("register must set session cookie")
        };
    };
}

macro_rules! post_create {
    ($app:expr, $jwt:expr, $payload:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/storage")
            .cookie($jwt.clone())
            .set_json($payload)
            .to_request();
        test::call_service(&$app, req).await
    }};
}

#[actix_web::test]
async fn instance_quota_rejects_create_over_ceiling() {
    let mut context = context::Context::mock_sqlite().await;
    context.config.app.storage_instance_quota_bytes = Some(1024);
    let app = test::init_service(server::app(context.clone())).await;
    register!(app, jwt, "alice@example.com");

    // First 1 KiB file sits exactly at the ceiling.
    let first = post_create!(app, jwt, sized_create("hash-1", &[1024]));
    assert_eq!(first.status(), StatusCode::OK);

    // Second would push the instance past 1 KiB.
    let second = post_create!(app, jwt, sized_create("hash-2", &[1024]));
    assert_eq!(second.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(second).await;
    assert_eq!(body["message"], "quota_exceeded");
}

#[actix_web::test]
async fn instance_quota_concurrent_creates_admit_exactly_one() {
    let mut context = context::Context::mock_sqlite_shared().await;
    context.config.app.storage_instance_quota_bytes = Some(1024);
    let app = test::init_service(server::app(context.clone())).await;
    register!(app, jwt, "alice@example.com");

    let req1 = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(sized_create("hash-a", &[1024]))
        .to_request();
    let req2 = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(sized_create("hash-b", &[1024]))
        .to_request();

    let (r1, r2) = tokio::join!(
        test::call_service(&app, req1),
        test::call_service(&app, req2),
    );

    let statuses = [r1.status(), r2.status()];
    assert_eq!(
        statuses.iter().filter(|s| s.is_success()).count(),
        1,
        "exactly one create may win the instance ceiling"
    );
    assert_eq!(
        statuses.iter().filter(|s| s.as_u16() == 400).count(),
        1,
        "the losing create must be rejected"
    );

    // The reserve lock must keep the instance at one file, never two.
    let total: i64 = files::Entity::find()
        .all(&context.db)
        .await
        .unwrap()
        .iter()
        .filter_map(|f| f.size)
        .sum();
    assert_eq!(
        total, 1024,
        "instance usage must not exceed the ceiling under a concurrent race"
    );
}

#[actix_web::test]
async fn instance_quota_rejects_tar_pre_read() {
    let mut context = context::Context::mock_with_data_dir(Some(
        "../data-test-instq-preread".to_string(),
    ))
    .await;
    context.config.app.storage_instance_quota_bytes = Some(2048);
    let app = test::init_service(server::app(context.clone())).await;
    register!(app, jwt, "alice@example.com");

    // Create reserves the full 2 KiB up front, sitting at the ceiling.
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(sized_create("tar-preread", &[1024, 1024]))
        .to_request();
    let file: AppFile = serde_json::from_slice(&test::call_and_read_body(&app, req).await)
        .expect("create AppFile");

    // A Content-Length advertises the incoming bytes, so the pre-read gate
    // rejects before a single chunk is streamed.
    let tar = tar_of(&[(0, &[7u8; 1024]), (1, &[7u8; 1024])]);
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?format=tar", file.id).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/x-tar"))
        .append_header(("content-length", tar.len()))
        .set_payload(tar)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "quota_exceeded");

    context.config.app.cleanup();
}

#[actix_web::test]
async fn instance_quota_rejects_tar_mid_stream() {
    let mut context = context::Context::mock_with_data_dir(Some(
        "../data-test-instq-midstream".to_string(),
    ))
    .await;
    context.config.app.storage_instance_quota_bytes = Some(3000);
    let app = test::init_service(server::app(context.clone())).await;
    register!(app, jwt, "alice@example.com");

    // Create reserves 2 KiB, under the 3000-byte ceiling.
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(sized_create("tar-midstream", &[1024, 1024]))
        .to_request();
    let file: AppFile = serde_json::from_slice(&test::call_and_read_body(&app, req).await)
        .expect("create AppFile");

    // No Content-Length, so the pre-read gate is skipped and the running-total
    // check inside the stream is the one that must catch the overflow: 3000 -
    // 2048 reserved = 952 bytes of headroom, short of the first 1 KiB chunk.
    let tar = tar_of(&[(0, &[7u8; 1024]), (1, &[7u8; 1024])]);
    let req = test::TestRequest::post()
        .uri(format!("/api/storage/{}?format=tar", file.id).as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/x-tar"))
        .set_payload(tar)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "quota_exceeded");

    context.config.app.cleanup();
}
