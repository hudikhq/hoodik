//! Client side of the `POST /api/storage/{file_id}?format=tar` endpoint.
//!
//! The single-HTTP-call counterpart to the per-chunk upload pipeline in
//! [`crate::upload`]. On slow networks one tar is measurably faster than N
//! per-chunk requests; the server unpacks it through the same write
//! primitives either way, so the on-disk result is identical.

use crate::error::{Error, Result};
use crate::platform::{HttpClient, ProgressReporter};
use crate::types::{Auth, ChunkResponse};

/// Pack in-memory encrypted chunks into a tar archive and POST it to the
/// bulk-upload endpoint.
///
/// Progress is reported as a single `on_chunk_downloaded` tick carrying the
/// total bytes going into the tar. The downloaded-bytes channel is reused
/// (not the per-chunk upload channel) because this path doesn't emit
/// per-entry events — one honest total-size tick beats several fake
/// "partial sent" ones. Cancellation is checked right before the HTTP send
/// so the signal stops the pipeline before it commits network bandwidth.
pub async fn upload_chunks_as_tar_in_memory(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    entries: Vec<(String, Vec<u8>)>,
) -> Result<ChunkResponse> {
    let chunk_count = entries.len() as u64;
    let bytes_read: u64 = entries.iter().map(|(_, d)| d.len() as u64).sum();

    let archive = crate::tar::build_tar(&entries);

    if progress.is_cancelled(file_id) {
        return Err(Error::Cancelled);
    }

    progress.on_chunk_downloaded(file_id, bytes_read, archive.len() as u64);

    let resp = http
        .upload_chunks_tar(auth, file_id, archive)
        .await
        .inspect_err(|e| progress.on_error(file_id, &format!("{e}")))?;

    let stored = resp.chunks_stored.unwrap_or(0) as u64;
    if stored >= chunk_count {
        progress.on_complete(file_id);
    }
    Ok(resp)
}

/// Upload chunks from a local directory as a single tar archive.
///
/// Reads `{chunks_dir}/{index:06}.enc` for each index `0..chunk_count` and
/// delegates to [`upload_chunks_as_tar_in_memory`]. Native/mobile only
/// because WASM targets can't open files by path.
#[cfg(any(feature = "native", feature = "mobile"))]
pub async fn upload_chunks_as_tar(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    chunks_dir: &str,
    chunk_count: u64,
) -> Result<ChunkResponse> {
    let mut entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(chunk_count as usize);

    for idx in 0..chunk_count {
        if progress.is_cancelled(file_id) {
            return Err(Error::Cancelled);
        }

        let path = format!("{}/{:06}.enc", chunks_dir, idx);
        let data = tokio::fs::read(&path)
            .await
            .map_err(|e| Error::Io(format!("read {}: {e}", path)))?;

        entries.push((format!("{:06}.enc", idx), data));
    }

    upload_chunks_as_tar_in_memory(http, progress, auth, file_id, entries).await
}
