//! Bulk chunk upload via a single tar stream.
//!
//! Accepts a ustar archive whose entries are named `{chunk_index:06}.enc` and
//! contain the already-encrypted chunk bodies. Each entry is unpacked directly
//! into the file's server-side chunk storage through the same `push`/`push_v`
//! primitives the per-chunk endpoint uses, so the on-disk result is
//! indistinguishable between the two paths. Finalization (pointer swap on the
//! versioned path, `finished_upload_at` stamp on both) runs once the stored
//! chunk count reaches the file's target — identical to the per-chunk flow.
//!
//! The tar path intentionally differs from per-chunk uploads on two points:
//!   * No CRC16 is checked per entry. Transport corruption is caught by TLS
//!     and by tar's own header checksum; the file-level SHA-256 submitted at
//!     the end of the upload is the source of truth for integrity.
//!   * Duplicate chunk indices inside one request are a hard 400 (corrupt
//!     input from the client). A chunk already on disk from a prior request
//!     is silently overwritten, matching normal chunked-resume semantics.

use actix_web::{web, HttpRequest, HttpResponse};
use auth::data::transfer_claims::StorageClaims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::{
    prelude::*,
    tar::{parse_next_entry, TarStep},
    MAX_CHUNK_SIZE_BYTES,
};
use futures::StreamExt;
use std::collections::HashSet;
use std::str::FromStr;

use crate::{
    data::app_file::AppFile,
    repository::{
        cached::{evict_file, get_file},
        Repository,
    },
};

/// Hard cap on how many bytes we'll hold in the stream-parser buffer before
/// declaring the request malformed. A compliant request never needs more than
/// one header (512 B) + one padded chunk payload (≤ MAX_CHUNK_SIZE_BYTES + 512)
/// in flight at once. The generous headroom absorbs header-straddle reads and
/// any residual bytes behind the current entry without ballooning memory.
const MAX_BUFFER_BYTES: usize = (MAX_CHUNK_SIZE_BYTES as usize) + 16 * 1024;

/// Handle `POST /api/storage/{file_id}?format=tar`. Dispatched from
/// [`super::upload::upload`] when the `format` query is set.
pub(crate) async fn upload_tar(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
    body: web::Payload,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    claims.validate_transfer_path(file_id, "upload")?;

    let file = get_file(&context, claims.sub(), file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    let target_chunks = file
        .target_chunks()
        .ok_or_else(|| Error::BadRequest("file_has_no_chunks".to_string()))?;

    // Enforce the quota before streaming the body. When Content-Length is
    // missing (e.g. `Transfer-Encoding: chunked`), the running-total check
    // inside [`stream_tar_into_storage`] catches over-quota mid-stream.
    let advertised_body_len = content_length_header(&req);
    if let Some(advertised) = advertised_body_len {
        enforce_quota_pre_read(&context, &claims, advertised).await?;
    }

    let versioned = file.use_versioned_layout();
    let storage = Fs::new(&context.config);

    let remaining_quota = remaining_quota_bytes(&context, &claims).await;

    let mut file = stream_tar_into_storage(
        body,
        &storage,
        &file,
        target_chunks,
        versioned,
        remaining_quota,
    )
    .await?;

    // Refresh stored-chunk bookkeeping from disk: the tar may have filled the
    // file completely, or only added a subset, and the legacy layout can't
    // track it in memory without a listing.
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
        file = finalize_file(&context, &storage, &claims, &file, file_id).await?;
    }

    Ok(HttpResponse::Ok().json(file))
}

/// Consume the tar stream end-to-end, writing each entry's body into the
/// backing storage provider. The incremental buffer pattern (append-bytes,
/// parse-at-front, drop-consumed) keeps peak memory at one chunk-sized
/// payload regardless of how many chunks the archive contains.
async fn stream_tar_into_storage(
    mut body: web::Payload,
    storage: &Fs<'_>,
    file: &AppFile,
    target_chunks: i64,
    versioned: bool,
    mut remaining_quota: Option<i64>,
) -> AppResult<AppFile> {
    let mut buffer: Vec<u8> = Vec::with_capacity(MAX_BUFFER_BYTES.min(1024 * 1024));
    let mut seen: HashSet<i64> = HashSet::new();
    let mut saw_end = false;

    while let Some(chunk) = body.next().await {
        let bytes = chunk.map_err(|e| Error::BadRequest(format!("payload_read_failed: {e}")))?;
        buffer.extend_from_slice(&bytes);

        loop {
            match parse_next_entry(&buffer) {
                TarStep::Entry {
                    name,
                    data,
                    consumed,
                } => {
                    let chunk_index = parse_chunk_name(&name)?;
                    validate_chunk_index(chunk_index, target_chunks)?;
                    reject_oversize_chunk(data.len())?;
                    if !seen.insert(chunk_index) {
                        return Err(Error::as_validation(
                            "chunk",
                            "duplicate_chunk_index_in_tar",
                        ));
                    }

                    if let Some(ref mut left) = remaining_quota {
                        *left = left.saturating_sub(data.len() as i64);
                        if *left < 0 {
                            return Err(Error::BadRequest("quota_exceeded".to_string()));
                        }
                    }

                    if versioned {
                        storage
                            .push_v(file, file.target_version(), chunk_index, data)
                            .await?;
                    } else {
                        storage.push(file, chunk_index, data).await?;
                    }

                    buffer.drain(..consumed);
                }
                TarStep::End => {
                    saw_end = true;
                    buffer.clear();
                    break;
                }
                TarStep::NeedMoreData => break,
                TarStep::Malformed(reason) => {
                    return Err(Error::BadRequest(format!("tar_malformed: {reason}")));
                }
            }
        }

        if saw_end {
            break;
        }

        if buffer.len() > MAX_BUFFER_BYTES {
            return Err(Error::BadRequest(
                "tar_entry_exceeds_max_chunk_size".to_string(),
            ));
        }
    }

    if !saw_end {
        return Err(Error::BadRequest("tar_truncated".to_string()));
    }

    Ok(file.clone())
}

/// Parse `{chunk_index:06}.enc` out of an entry name. Rejects anything else so
/// a creative client can't smuggle arbitrary filenames past the `data_dir`
/// prefix — the parsed integer is the only part that ever leaves this
/// function.
fn parse_chunk_name(name: &str) -> AppResult<i64> {
    let stem = name.strip_suffix(".enc").ok_or_else(|| {
        Error::BadRequest(format!("tar_entry_not_enc_suffix: {name}"))
    })?;
    stem.parse::<i64>()
        .map_err(|_| Error::BadRequest(format!("tar_entry_not_integer_index: {name}")))
}

fn validate_chunk_index(chunk_index: i64, target_chunks: i64) -> AppResult<()> {
    if chunk_index < 0 || chunk_index >= target_chunks {
        return Err(Error::as_validation("chunk", "chunk_out_of_range"));
    }
    Ok(())
}

fn reject_oversize_chunk(len: usize) -> AppResult<()> {
    if len as u64 > MAX_CHUNK_SIZE_BYTES {
        return Err(Error::as_validation("chunk", "chunk_size_exceeds_max"));
    }
    Ok(())
}

fn content_length_header(req: &HttpRequest) -> Option<u64> {
    req.headers()
        .get(actix_web::http::header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse::<u64>()
        .ok()
}

/// Reject up-front when the advertised body won't fit in the remaining quota.
/// Silently skipped if the claims don't carry a quota (transfer tokens, for
/// example) — the running-total check inside the stream loop is the safety
/// net in that case.
async fn enforce_quota_pre_read(
    context: &Context,
    claims: &StorageClaims,
    advertised: u64,
) -> AppResult<()> {
    let quota = match claims.get_quota(context).await {
        Some(q) => q,
        None => return Ok(()),
    };
    let used = Repository::new(&context.db)
        .query(claims.sub())
        .used_space()
        .await?;
    if used.saturating_add(advertised as i64) > quota as i64 {
        return Err(Error::BadRequest("quota_exceeded".to_string()));
    }
    Ok(())
}

/// Remaining bytes the caller is allowed to add before running into their
/// quota. `None` means "no quota", e.g. transfer tokens — in which case the
/// running-total check is a no-op.
async fn remaining_quota_bytes(context: &Context, claims: &StorageClaims) -> Option<i64> {
    let quota = claims.get_quota(context).await?;
    let used = Repository::new(&context.db)
        .query(claims.sub())
        .used_space()
        .await
        .unwrap_or(0);
    Some((quota as i64).saturating_sub(used))
}

/// Same transactional commit the per-chunk path runs: snapshot + swap +
/// prune in one DB transaction, then best-effort-purge the pruned version
/// directories. See [`super::upload::upload`] for the long-form rationale.
async fn finalize_file(
    context: &Context,
    storage: &Fs<'_>,
    claims: &StorageClaims,
    file: &AppFile,
    file_id: Uuid,
) -> AppResult<AppFile> {
    use entity::TransactionTrait;
    let txn = context.db.begin().await?;
    let (mut finished_file, pruned) = Repository::new(&txn)
        .manage(claims.sub())
        .finish(file, context.config.app.max_file_versions)
        .await?;
    txn.commit().await?;

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
    finished_file.uploaded_chunks = file.uploaded_chunks.clone();
    Ok(finished_file)
}
