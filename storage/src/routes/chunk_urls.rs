//! Presigned chunk URLs, so clients move ciphertext straight to and from the
//! storage bucket instead of through this server.
//!
//! These routes hand out capabilities rather than bytes, which makes them the
//! only authorization gate on the direct path. Each one runs exactly the check
//! its byte-serving counterpart runs — not an approximation of it.
//!
//! Every route 400s when the provider cannot issue URLs, which is the answer
//! for local-filesystem deployments and for any S3 deployment whose bucket
//! failed the checks in `fs::direct`. Clients read `direct_transfer` from
//! `/api/capabilities` and use the relaying routes instead; a client that asks
//! anyway gets told plainly rather than getting a broken transfer.

use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::transfer_claims::StorageClaims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::prelude::*;

use crate::{
    data::chunk_urls::{ChunkUrls, UploadUrlsRequest},
    permission::{require_read, require_write},
    repository::{cached::get_file, Repository},
};

/// Unix time after which URLs signed now stop working.
fn expires_at(context: &Context) -> i64 {
    let seconds = context
        .config
        .s3
        .as_ref()
        .map(|s3| s3.direct_expiry_secs)
        .unwrap_or(0);

    chrono::Utc::now().timestamp() + seconds as i64
}

fn unavailable() -> Error {
    Error::BadRequest("direct_transfer_unavailable".to_string())
}

/// `GET /api/storage/{file_id}/chunk-urls` — read URLs for every stored chunk
/// of a file's active version.
///
/// Authorization mirrors [`super::download::download`] exactly.
#[route("/api/storage/{file_id}/chunk-urls", method = "GET")]
pub(crate) async fn download_urls(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    claims.validate_transfer_path(file_id, "download")?;

    // Authorize against the database rather than leaning on the cache: this
    // route hands out working presigned URLs, so its read gate has to be as
    // explicit as the one on `version_urls`.
    require_read(&context.db, file_id, claims.sub()).await?;

    let file = get_file(&context, claims.sub(), file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    let storage = Fs::new(&context.config);
    let version = file.active_version;

    let chunks = if file.use_versioned_layout() {
        storage.get_uploaded_chunks_v(&file, version).await?
    } else {
        storage.get_uploaded_chunks(&file).await?
    };

    let urls = storage
        .direct_get_urls(&file, version, &chunks)
        .await?
        .ok_or_else(unavailable)?;

    Ok(HttpResponse::Ok().json(ChunkUrls::new(&chunks, urls, expires_at(&context))))
}

/// `POST /api/storage/{file_id}/upload-urls` — write URLs for the chunks a
/// client is about to push into the file's target version.
///
/// The body declares each chunk's byte length. Those lengths are charged
/// against the quota here and signed into the URLs, so this is where both
/// limits that the relaying upload route enforces per request get applied
/// instead.
#[route("/api/storage/{file_id}/upload-urls", method = "POST")]
pub(crate) async fn upload_urls(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
    body: web::Json<UploadUrlsRequest>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    claims.validate_transfer_path(file_id, "upload")?;
    require_write(&context.db, file_id, claims.sub()).await?;

    let file = get_file(&context, claims.sub(), file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    let target_chunks = file
        .target_chunks()
        .ok_or_else(|| Error::BadRequest("file_has_no_chunks".to_string()))?;

    let mut requested: Vec<(i64, u64)> = Vec::with_capacity(body.chunks.len());
    let mut seen = std::collections::HashSet::new();
    let mut declared_total: u64 = 0;

    for pending in &body.chunks {
        super::upload_tar::validate_chunk_index(pending.chunk, target_chunks)?;
        if !seen.insert(pending.chunk) {
            return Err(Error::as_validation("chunk", "duplicate_chunk_index"));
        }
        if pending.size > fs::MAX_CHUNK_PAYLOAD_BYTES {
            return Err(Error::as_validation("chunk", "chunk_size_exceeds_max"));
        }
        declared_total = declared_total.saturating_add(pending.size);
        requested.push((pending.chunk, pending.size));
    }

    // The client writes straight into the bucket after this, so nothing later
    // in the upload can refuse it for being over quota. Charge the declared
    // total now; `finalize` re-checks against what actually landed.
    super::upload_tar::enforce_quota_pre_read(&context, &claims, declared_total).await?;

    let storage = Fs::new(&context.config);
    let version = file.target_version();

    let urls = storage
        .direct_put_urls(&file, version, &requested)
        .await?
        .ok_or_else(unavailable)?;

    let indices: Vec<i64> = requested.iter().map(|(chunk, _)| *chunk).collect();
    Ok(HttpResponse::Ok().json(ChunkUrls::new(&indices, urls, expires_at(&context))))
}

/// `POST /api/storage/{file_id}/finalize` — commit a file whose chunks the
/// client wrote directly.
///
/// The relaying routes finalize inline once their own writes complete the
/// count. Nothing tells this server when a direct write lands, so the client
/// says so and the bucket is asked to confirm it: a listing has to show every
/// chunk before the version pointer moves.
#[route("/api/storage/{file_id}/finalize", method = "POST")]
pub(crate) async fn finalize(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    claims.validate_transfer_path(file_id, "upload")?;
    require_write(&context.db, file_id, claims.sub()).await?;

    let mut file = get_file(&context, claims.sub(), file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    let target_chunks = file
        .target_chunks()
        .ok_or_else(|| Error::BadRequest("file_has_no_chunks".to_string()))?;

    let storage = Fs::new(&context.config);
    let versioned = file.use_versioned_layout();

    let stored = if versioned {
        storage
            .get_uploaded_chunks_v(&file, file.target_version())
            .await?
    } else {
        storage.get_uploaded_chunks(&file).await?
    };

    if (stored.len() as i64) < target_chunks {
        return Err(Error::as_validation("chunks", "chunks_missing"));
    }

    file.chunks_stored = Some(stored.len() as i64);
    file.uploaded_chunks = Some(stored);

    let file =
        super::upload_tar::finalize_file(&context, &storage, &claims, &file, file_id).await?;

    Ok(HttpResponse::Ok().json(file))
}

/// `GET /api/storage/{file_id}/versions/{version}/chunk-urls` — read URLs for
/// a historical version.
///
/// Authorization mirrors [`super::versions::download`], including the
/// existence check on the version row.
#[route(
    "/api/storage/{file_id}/versions/{version}/chunk-urls",
    method = "GET"
)]
pub(crate) async fn version_urls(
    req: HttpRequest,
    claims: auth::data::claims::Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let version: i32 = util::actix::path_var(&req, "version")?;

    crate::permission::require_read(&context.db, file_id, claims.sub).await?;

    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .file(file_id)
        .await?;
    Repository::new(&context.db)
        .versions(claims.sub)
        .get(file_id, version)
        .await?;

    let storage = Fs::new(&context.config);
    let chunks = storage.get_uploaded_chunks_v(&file, version).await?;

    let urls = storage
        .direct_get_urls(&file, version, &chunks)
        .await?
        .ok_or_else(unavailable)?;

    Ok(HttpResponse::Ok().json(ChunkUrls::new(&chunks, urls, expires_at(&context))))
}
