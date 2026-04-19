//! Version-history endpoints for editable files.
//!
//! All operations are owner-only — there's no transfer-token surface
//! here, intentionally: history access is metadata + restore, neither of
//! which the upload/download proxy flow needs.

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use chrono::Utc;
use context::Context;
use entity::{ActiveValue, TransactionTrait, Uuid};
use error::AppResult;
use fs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    data::create_file::CreateFile,
    repository::{cached::evict_file, Repository},
};

/// `GET /api/storage/{file_id}/versions` — newest-first list of
/// historical snapshots. The currently-active version is intentionally
/// absent (it lives on the file row, not in `file_versions`).
#[route("/api/storage/{file_id}/versions", method = "GET")]
pub(crate) async fn list(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    let versions = Repository::new(&context.db)
        .versions(claims.sub)
        .list(file_id)
        .await?;

    Ok(HttpResponse::Ok().json(versions))
}

/// `GET /api/storage/{file_id}/versions/{version}` — fetch chunks of a
/// historical version. Mirrors the regular download endpoint:
/// `?format=tar` for a tar of all chunks, `?chunk=N` for a single chunk.
#[route("/api/storage/{file_id}/versions/{version}", method = "GET")]
pub(crate) async fn download(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let version: i32 = util::actix::path_var(&req, "version")?;
    let chunk = util::actix::query_var::<i64>(&req, "chunk").ok();
    let format = util::actix::query_var::<String>(&req, "format").ok();

    // Authorization + existence go through the repository — both file
    // and historical version must exist for the calling owner.
    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(file_id)
        .await?;
    let _ = Repository::new(&context.db)
        .versions(claims.sub)
        .get(file_id, version)
        .await?;

    let storage = Fs::new(&context.config);

    if format.as_deref() == Some("tar") {
        let content_length = storage.tar_content_length_v(&file, version).await?;
        let streamer = storage.stream_tar_v(&file, version).await?;
        let filename = format!("{}.v{}.tar", file_id, version);

        return Ok(HttpResponse::Ok()
            .insert_header(("Content-Type", "application/x-tar"))
            .insert_header(("Content-Length", content_length.to_string()))
            .insert_header((
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ))
            .streaming(streamer.stream()));
    }

    let streamer = storage.stream_v(&file, version, chunk).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/octet-stream"))
        .streaming(streamer.stream()))
}

/// `POST /api/storage/{file_id}/versions/{version}/restore` —
/// in-place restore. Allocates a new version slot, copies the target's
/// chunks into it, and flips `active_version`. The previously-active
/// version is snapshotted into history along the way, so the user can
/// undo by restoring that one.
#[route(
    "/api/storage/{file_id}/versions/{version}/restore",
    method = "POST"
)]
pub(crate) async fn restore(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let version: i32 = util::actix::path_var(&req, "version")?;

    let storage = Fs::new(&context.config);

    let txn = context.db.begin().await?;
    let outcome = Repository::new(&txn)
        .versions(claims.sub)
        .restore(file_id, version)
        .await?;
    txn.commit().await?;

    // Pre-load the file via a fresh read so the chunks-copy below uses
    // the post-swap `IntoFilename` (timestamp/UUID unchanged, but it's
    // the right contract).
    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(file_id)
        .await?;

    // Copy chunks out of band so the DB transaction stays small. If
    // copy fails the active pointer already moved — same recovery
    // story as a half-finished edit: the next purge_all on file delete
    // catches the dst dir, and a re-restore retries cleanly.
    storage
        .copy_version(&file, outcome.source_version, &file, outcome.new_version)
        .await?;

    evict_file(file_id).await;

    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(file_id)
        .await?;
    Ok(HttpResponse::Ok().json(file))
}

/// Body for `fork`. Uses the `CreateFile` shape because the client
/// builds the new file's encrypted metadata exactly as it would for a
/// fresh upload — only the chunks are server-copied.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForkRequest {
    /// Same fields as `createFile` — client-encrypted name/thumbnail,
    /// the file's symmetric key encrypted under the user's RSA pubkey,
    /// destination directory, etc. `chunks`/`size` are ignored here:
    /// the source version's recorded values are authoritative.
    #[serde(flatten)]
    pub file: CreateFile,
}

/// `POST /api/storage/{file_id}/versions/{version}/fork` —
/// restore-as-new-note. Creates a brand-new file row reusing the
/// source's chunks (same encryption key) and metadata supplied by the
/// client. The original file is untouched.
#[route("/api/storage/{file_id}/versions/{version}/fork", method = "POST")]
pub(crate) async fn fork(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<ForkRequest>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let source_file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let source_version: i32 = util::actix::path_var(&req, "version")?;

    let source = Repository::new(&context.db)
        .manage(claims.sub)
        .file(source_file_id)
        .await?;

    // Source chunks/size/sha256 come from `file_versions` for historical
    // versions, or directly off the file row for the active one (no
    // history row exists for active until its first edit).
    let (chunks, size, sha256) = if source_version == source.active_version {
        (
            source.chunks.unwrap_or(0),
            source.size.unwrap_or(0),
            source.sha256.clone(),
        )
    } else {
        let v = Repository::new(&context.db)
            .versions(claims.sub)
            .get(source_file_id, source_version)
            .await?;
        (v.chunks, v.size, v.sha256)
    };

    // The client builds the new file's encrypted metadata; the server
    // overwrites the chunk-content fields with the source's recorded
    // values so the new row describes what's actually on disk.
    let mut payload = data.into_inner().file;
    payload.chunks = Some(chunks);
    payload.size = Some(size);
    payload.sha256 = sha256;

    let (mut active_model, encrypted_key, _hashed_tokens, _size, _parent) =
        payload.into_active_model()?;
    // Skip the chunk-upload pass — chunks land via copy_version below.
    let now = Utc::now().timestamp();
    active_model.chunks_stored = ActiveValue::Set(Some(chunks));
    active_model.finished_upload_at = ActiveValue::Set(Some(now));

    let storage = Fs::new(&context.config);

    let txn = context.db.begin().await?;
    let outcome = Repository::new(&txn)
        .versions(claims.sub)
        .fork(source_file_id, source_version, active_model, encrypted_key)
        .await?;
    txn.commit().await?;

    let new_file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(outcome.new_file_id)
        .await?;

    // Source chunks live under the original file's path; destination
    // is the brand-new file's v1.
    storage
        .copy_version(
            &source,
            outcome.source_version,
            &new_file,
            new_file.active_version,
        )
        .await?;

    Ok(HttpResponse::Ok().json(new_file))
}

/// `DELETE /api/storage/{file_id}/versions/{version}` — drop a single
/// historical version. Deleting the active version is rejected; use
/// regular file deletion for that.
#[route(
    "/api/storage/{file_id}/versions/{version}",
    method = "DELETE"
)]
pub(crate) async fn delete(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let version: i32 = util::actix::path_var(&req, "version")?;

    Repository::new(&context.db)
        .versions(claims.sub)
        .delete(file_id, version)
        .await?;

    // On-disk cleanup is best-effort — DB row already gone, so an
    // orphaned dir gets caught by the next purge_all on file delete.
    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(file_id)
        .await?;
    if let Err(e) = Fs::new(&context.config).purge_version(&file, version).await {
        log::warn!(
            "Failed to purge v{} dir for file {}: {}",
            version,
            file_id,
            e
        );
    }

    Ok(HttpResponse::NoContent().finish())
}

/// `DELETE /api/storage/{file_id}/versions` — drop every historical
/// version, keeping only the active one.
#[route("/api/storage/{file_id}/versions", method = "DELETE")]
pub(crate) async fn purge_all_history(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    let pruned = Repository::new(&context.db)
        .versions(claims.sub)
        .purge_all_history(file_id)
        .await?;

    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(file_id)
        .await?;
    let storage = Fs::new(&context.config);
    for version in pruned {
        if let Err(e) = storage.purge_version(&file, version).await {
            log::warn!(
                "Failed to purge v{} dir for file {}: {}",
                version,
                file_id,
                e
            );
        }
    }

    Ok(HttpResponse::NoContent().finish())
}

