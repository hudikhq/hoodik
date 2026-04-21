//! A2a — split routing between the legacy flat-layout path (non-editable
//! files) and the versioned `v{N}/` path (editable notes). These tests
//! pin the contract down at the HTTP surface: what lands on disk, what
//! comes back on download, and what `set_editable` rejects once a file
//! has an edit in flight or history accumulated.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use entity::{ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use std::path::Path;
use storage::data::app_file::AppFile;
use storage::data::set_editable::SetEditable;

use crate::helpers::{calculate_checksum, create_byte_chunks};

/// Build a `CreateUser` payload with a fresh keypair. Registration is
/// cookie-based, so the returned value is ready to drop into the HTTP
/// request that extracts the session cookie.
fn make_register(email: &str) -> CreateUser {
    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    CreateUser {
        email: Some(email.to_string()),
        password: Some("not-4-weak-password-for-god-sakes!".to_string()),
        secret: None,
        token: None,
        pubkey: Some(public_string),
        fingerprint: Some(fingerprint),
        encrypted_private_key: Some("encrypted-secret".to_string()),
        invitation_id: None,
    }
}

/// True iff the data dir contains at least one legacy `{timestamp}-{uuid}.part.{n}`
/// file for this file's id. Used to assert routing landed chunks where the
/// layout predicts they should be.
fn has_legacy_chunk(data_dir: &str, file: &AppFile) -> bool {
    let pattern = format!("{}/{}-{}.part.*", data_dir, file.created_at, file.id);
    glob::glob(&pattern)
        .map(|paths| paths.filter_map(Result::ok).next().is_some())
        .unwrap_or(false)
}

/// True iff the file has any `{file_id}/v{version}/*.chunk` entries on disk.
fn has_versioned_chunks(data_dir: &str, file: &AppFile, version: i32) -> bool {
    let dir = format!("{}/{}/v{}", data_dir, file.id, version);
    match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().ends_with(".chunk")),
        Err(_) => false,
    }
}

/// Non-editable files must land under the legacy flat layout — no
/// `{uuid}/v1/` directory, just `{timestamp}-{uuid}.part.{n}` files
/// directly under `data_dir`. Download has to round-trip the bytes.
#[actix_web::test]
async fn test_non_editable_file_uses_legacy_layout_on_disk() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data-test-legacy-routing-noneditable".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&make_register("legacy-routing-1@test.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let (data, size, _) = create_byte_chunks();
    let checksum = calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("binary.enc".to_string()),
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
        .set_json(&create)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let mut file: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(!file.editable);

    for (i, chunk) in data.iter().enumerate() {
        let cs = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!("/api/storage/{}?checksum={}&chunk={}", file.id, cs, i);
        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk.clone())
            .to_request();
        let body = test::call_and_read_body(&app, req).await;
        file = serde_json::from_slice(&body).unwrap();
    }
    assert!(file.finished_upload_at.is_some());

    let data_dir = context.config.app.data_dir.as_str();
    assert!(
        has_legacy_chunk(data_dir, &file),
        "non-editable upload must leave legacy `.part.n` chunks in data_dir"
    );
    assert!(
        !Path::new(&format!("{}/{}/v1", data_dir, file.id)).exists(),
        "non-editable upload must NOT create a `{{uuid}}/v1/` directory"
    );

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let bytes = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(
        bytes.len() as i64,
        size,
        "non-editable download must return the same byte count"
    );
    assert_eq!(cryptfns::sha256::digest(bytes.as_slice()), checksum);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// Editable notes must land under `{uuid}/v1/000000.chunk` and no legacy
/// `.part.0` should ever be written. Download has to round-trip the bytes.
#[actix_web::test]
async fn test_editable_note_uses_versioned_layout() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data-test-legacy-routing-editable".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&make_register("legacy-routing-2@test.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let payload = b"# hello world\n\nediting notes is nice".to_vec();
    let size = payload.len() as i64;
    let data = vec![payload.clone()];
    let checksum = calculate_checksum(data.clone());

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(checksum.clone()),
        mime: Some("text/markdown".to_string()),
        size: Some(size),
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
    assert!(file.editable);

    let cs = cryptfns::sha256::digest(payload.as_slice());
    let uri = format!("/api/storage/{}?checksum={}&chunk=0", file.id, cs);
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(payload.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(file.finished_upload_at.is_some());
    assert_eq!(file.active_version, 1);

    let data_dir = context.config.app.data_dir.as_str();
    assert!(
        has_versioned_chunks(data_dir, &file, 1),
        "editable upload must write into `{{uuid}}/v1/`"
    );
    assert!(
        !has_legacy_chunk(data_dir, &file),
        "editable upload must NOT write legacy `.part.n` chunks"
    );

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let bytes = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(bytes, payload, "editable download round-trips the content");

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// A file created as non-editable keeps its legacy chunks in place when
/// the flag flips to editable — `set_editable` doesn't touch disk. The
/// first subsequent edit snapshots the legacy chunks as v1 (via the
/// existing legacy-fallback in `copy_version`) and writes the new
/// content as v2. History for v=1 must reflect the original chunk count.
#[actix_web::test]
async fn test_set_editable_true_preserves_legacy_then_first_edit_snapshots_as_v1() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data-test-legacy-routing-flipthen-edit".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&make_register("legacy-routing-3@test.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let v1_payload = b"original legacy content v1".to_vec();
    let v1_size = v1_payload.len() as i64;
    let v1_checksum = calculate_checksum(vec![v1_payload.clone()]);

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("legacy-flip.md".to_string()),
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
        sha256: Some(v1_checksum.clone()),
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

    let cs = cryptfns::sha256::digest(v1_payload.as_slice());
    let uri = format!("/api/storage/{}?checksum={}&chunk=0", file.id, cs);
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v1_payload.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let data_dir = context.config.app.data_dir.as_str();
    assert!(has_legacy_chunk(data_dir, &file));

    // Flip to editable — must succeed, no disk I/O.
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/editable", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&SetEditable { editable: Some(true) })
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert!(file.editable);
    assert_eq!(file.active_version, 1);
    assert!(
        has_legacy_chunk(data_dir, &file),
        "set_editable(true) leaves legacy chunks untouched"
    );
    assert!(
        !has_versioned_chunks(data_dir, &file, 1),
        "set_editable(true) does not create a v1/ directory"
    );

    // Edit: allocate pending v2 and upload the new content.
    let v2_payload = b"edited content v2 looks very different".to_vec();
    let v2_size = v2_payload.len() as i64;

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({ "size": v2_size, "chunks": 1 }))
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.pending_version, Some(2));

    let cs = cryptfns::sha256::digest(v2_payload.as_slice());
    let uri = format!("/api/storage/{}?checksum={}&chunk=0", file.id, cs);
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v2_payload.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.active_version, 2, "first edit flips active to v2");
    assert!(file.pending_version.is_none());

    assert!(
        has_versioned_chunks(data_dir, &file, 2),
        "v2 chunks must live under `{{uuid}}/v2/`"
    );

    let history = entity::file_versions::Entity::find()
        .filter(entity::file_versions::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].version, 1);
    assert_eq!(history[0].chunks, 1);
    assert_eq!(history[0].size, v1_size);
    assert_eq!(history[0].sha256.as_deref(), Some(v1_checksum.as_str()));

    // GET /versions/1 must stream the original legacy bytes back intact.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/versions/1", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let bytes = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(
        bytes, v1_payload,
        "historical v=1 must serve the original legacy content via the versioned path (legacy fallback in stream_v)"
    );

    // Active download now serves v2.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let bytes = test::call_and_read_body(&app, req).await.to_vec();
    assert_eq!(bytes, v2_payload);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// Once a file has history (`active_version > 1` or any `file_versions` row),
/// reverting `editable → false` would orphan the versioned chunks — the legacy
/// read path can't see them. The guard returns 409, and deleting the history
/// rows doesn't help: `active_version > 1` is sufficient on its own. The
/// only recovery is deleting the file.
#[actix_web::test]
async fn test_set_editable_false_rejects_with_history() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data-test-legacy-routing-reject-history".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&make_register("legacy-routing-4@test.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    // Create editable and commit v1.
    let v1 = b"v1 payload".to_vec();
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("edit-me.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(calculate_checksum(vec![v1.clone()])),
        mime: Some("text/markdown".to_string()),
        size: Some(v1.len() as i64),
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

    let cs = cryptfns::sha256::digest(v1.as_slice());
    let uri = format!("/api/storage/{}?checksum={}&chunk=0", file.id, cs);
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v1.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.active_version, 1);

    // Edit once → active_version becomes 2, history row at v=1 is created.
    let v2 = b"v2 payload replaces v1".to_vec();
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({ "size": v2.len() as i64, "chunks": 1 }))
        .to_request();
    let _ = test::call_and_read_body(&app, req).await;

    let cs = cryptfns::sha256::digest(v2.as_slice());
    let uri = format!("/api/storage/{}?checksum={}&chunk=0", file.id, cs);
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v2.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.active_version, 2);

    // First rejection: history row + active>1.
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/editable", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&SetEditable { editable: Some(false) })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = test::read_body(resp).await;
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["message"].as_str(),
        Some("cannot_disable_editable_with_history")
    );

    // Drop the history rows and try again. active_version > 1 is still
    // sufficient on its own — the only safe recovery is deleting the file.
    let req = test::TestRequest::delete()
        .uri(format!("/api/storage/{}/versions", file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/editable", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&SetEditable { editable: Some(false) })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "active_version > 1 alone blocks the flip; the file has to be deleted to revert"
    );

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// Flipping `editable` while an edit is in flight would strand the pending
/// version's chunks — the guard returns 409 regardless of direction.
#[actix_web::test]
async fn test_set_editable_during_pending_edit_is_409() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data-test-legacy-routing-pending-409".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&make_register("legacy-routing-5@test.com"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let v1 = b"v1 payload".to_vec();
    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("pending.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(calculate_checksum(vec![v1.clone()])),
        mime: Some("text/markdown".to_string()),
        size: Some(v1.len() as i64),
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

    let cs = cryptfns::sha256::digest(v1.as_slice());
    let uri = format!("/api/storage/{}?checksum={}&chunk=0", file.id, cs);
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(v1.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    // Allocate a pending edit but DON'T upload the chunks.
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&serde_json::json!({ "size": 100, "chunks": 1 }))
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();
    assert_eq!(file.pending_version, Some(2));

    // Try to flip editable → false mid-edit.
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/editable", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&SetEditable { editable: Some(false) })
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = test::read_body(resp).await;
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        payload["message"].as_str(),
        Some("cannot_change_editable_during_edit")
    );

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}
