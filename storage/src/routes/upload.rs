use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::transfer_claims::StorageClaims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::{prelude::*, MAX_CHUNK_SIZE_BYTES};
use futures::StreamExt;

use crate::{
    data::{app_file::AppFile, meta::Meta},
    repository::{
        cached::{evict_file, get_file},
        Repository,
    },
};

/// Per-chunk upload ceiling — one chunk plus the 1% slack the client is
/// allowed to add on top of [`MAX_CHUNK_SIZE_BYTES`]. Shared between the
/// chunk-size validator and the manual body collector so a single constant
/// governs "how much we'll accept for one chunk".
const MAX_CHUNK_PAYLOAD_BYTES: u64 =
    MAX_CHUNK_SIZE_BYTES + MAX_CHUNK_SIZE_BYTES / 100;

/// Upload one chunk (per-chunk mode, legacy) or a tar archive of many
/// chunks (`?format=tar`, bulk mode).
///
/// Per-chunk request:
///   * Query: [`Meta`] (chunk index, optional checksum).
///   * Body: raw ciphertext for the chunk.
///
/// Bulk tar request (see [`super::upload_tar`]):
///   * Query: `format=tar`.
///   * Body: ustar archive whose entries are named `{chunk_index:06}.enc`.
///
/// Response: [`AppFile`] with updated chunk counters and, once the final
/// chunk has landed, the swapped-in active version and `finished_upload_at`
/// stamp.
#[route("/api/storage/{file_id}", method = "POST")]
pub(crate) async fn upload(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
    body: web::Payload,
) -> AppResult<HttpResponse> {
    if util::actix::query_var::<String>(&req, "format").ok().as_deref() == Some("tar") {
        return super::upload_tar::upload_tar(req, claims, context, body).await;
    }

    let meta = match web::Query::<Meta>::from_query(req.query_string()) {
        Ok(q) => q.into_inner(),
        Err(e) => return Err(Error::BadRequest(format!("invalid_query: {e}"))),
    };

    let request_body = collect_chunk_body(body).await?;
    upload_one_chunk(req, claims, context, meta, request_body).await
}

/// Classic single-chunk upload path, factored out so `upload` can stay a
/// thin dispatcher. Behaviour is identical to the pre-tar implementation:
/// checksum-verify, reject duplicates, push via the appropriate versioned
/// API, then auto-finalize if the write completed the file.
async fn upload_one_chunk(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
    meta: Meta,
    mut request_body: web::Bytes,
) -> AppResult<HttpResponse> {
    if request_body.is_empty() {
        return Err(Error::BadRequest("no_file_data_received".to_string()));
    }

    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    claims.validate_transfer_path(file_id, "upload")?;
    let (chunk, checksum, checksum_function, key_hex) = meta.into_tuple()?;

    validate_checksum(checksum, checksum_function, &request_body)?;

    if let Some(key) = key_hex {
        request_body = encrypt_request_body(&key, request_body)?;
    }

    let storage = Fs::new(&context.config);

    let mut file = get_file(&context, claims.sub(), file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    validate_chunk_size(&file, chunk, request_body.len())?;

    let versioned = file.use_versioned_layout();

    // Target version only matters on the versioned path. Non-editable
    // files never snapshot/swap; their chunks live directly in the
    // legacy flat layout and finalize only stamps `finished_upload_at`.
    let target_chunks = file
        .target_chunks()
        .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

    if chunk > target_chunks {
        return Err(Error::as_validation("chunk", "chunk_out_of_range"));
    }

    let chunk_exists = if versioned {
        storage.exists_v(&file, file.target_version(), chunk).await?
    } else {
        storage.exists(&file, chunk).await?
    };
    if chunk_exists {
        return Err(Error::as_validation("chunk", "chunk_already_exists"));
    }

    if versioned {
        storage
            .push_v(&file, file.target_version(), chunk, &request_body)
            .await?;
    } else {
        storage.push(&file, chunk, &request_body).await?;
    }

    if file.is_file() {
        let stored = if versioned {
            storage
                .get_uploaded_chunks_v(&file, file.target_version())
                .await?
        } else {
            storage.get_uploaded_chunks(&file).await?
        };

        file.chunks_stored = Some(stored.len() as i64);
        file.uploaded_chunks = Some(stored);
    }

    if file.chunks_stored == Some(target_chunks) {
        // On the versioned (edit) path, snapshot insert + pointer swap +
        // prune commit together so a crash mid-finalize can't leave history
        // pointing at a version that doesn't exist. On the non-versioned
        // path `finish` only stamps `finished_upload_at` — the transaction
        // is redundant there, but keeping it uniform avoids a branch.
        use entity::TransactionTrait;
        let txn = context.db.begin().await?;
        let (mut finished_file, pruned) = Repository::new(&txn)
            .manage(claims.sub())
            .finish(&file, context.config.app.max_file_versions)
            .await?;
        txn.commit().await?;

        // Stale cache would point any immediate follow-up read at the
        // pre-swap version.
        evict_file(file_id).await;

        for version in pruned {
            if let Err(e) = storage.purge_version(&finished_file, version).await {
                log::warn!(
                    "Failed to purge pruned v{} for file {}: {}. \
                     On-disk garbage; caught by next purge_all on file delete.",
                    version,
                    file_id,
                    e
                );
            }
        }

        finished_file.chunks_stored = file.chunks_stored;
        finished_file.uploaded_chunks = file.uploaded_chunks;
        file = finished_file;
    }

    Ok(HttpResponse::Ok().json(file))
}

/// Read the raw payload stream into memory, bounded to a single chunk's
/// worth of bytes. The global `PayloadConfig` used to cap this via the
/// `web::Bytes` extractor; with `web::Payload` the cap lives here so the
/// ceiling stays the same as before the tar-upload dispatch landed.
async fn collect_chunk_body(mut body: web::Payload) -> AppResult<web::Bytes> {
    let mut buf = web::BytesMut::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| Error::BadRequest(format!("payload_read_failed: {e}")))?;
        if (buf.len() + chunk.len()) as u64 > MAX_CHUNK_PAYLOAD_BYTES {
            return Err(Error::as_validation(
                "chunk",
                "chunk_size_exceeds_max",
            ));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf.freeze())
}

fn validate_checksum(
    checksum: Option<String>,
    checksum_function: Option<String>,
    data: &[u8],
) -> AppResult<()> {
    let body_checksum = match checksum_function.as_deref() {
        Some("crc16") => Some(cryptfns::crc::crc16_digest(data)),
        Some("sha256") => Some(cryptfns::sha256::digest(data)),
        _ => None,
    };

    if let Some(body_checksum) = body_checksum.as_deref() {
        if checksum.as_deref() != Some(body_checksum) {
            let error = format!(
                "checksum_mismatch: '{}' != '{}'",
                checksum.unwrap_or_default(),
                body_checksum
            );

            return Err(Error::as_validation("checksum", &error));
        }
    } else {
        log::warn!("Not validating uploaded chunk checksum");
    }

    Ok(())
}

/// Server-side encryption fallback for clients that can't encrypt locally
/// (embedded devices, scripts). Off by default — setting `key_hex` in the
/// query is the client's explicit opt-in. Chunk data is trusted as-is
/// otherwise; the integrity contract is the file-level hash the client
/// submits at the end of the upload.
fn encrypt_request_body(key: &str, request_body: web::Bytes) -> AppResult<web::Bytes> {
    let key = cryptfns::hex::decode(key)?;
    let encrypted = cryptfns::aes::encrypt(key, request_body.to_vec())?;

    Ok(web::Bytes::from(encrypted))
}

/// Reject bodies visibly larger than one chunk. The 1% tolerance matches
/// the historical behaviour of the route — a bit of slack for AEAD tags
/// and any future padding/header the cipher layer might add.
fn validate_chunk_size(_file: &AppFile, _chunk: i64, data_len: usize) -> AppResult<()> {
    if data_len as u64 > MAX_CHUNK_PAYLOAD_BYTES {
        let error = format!(
            "chunk_size_mismatch: expected max {MAX_CHUNK_PAYLOAD_BYTES}, but received {data_len}"
        );
        return Err(Error::as_validation("chunk", &error));
    }
    Ok(())
}
