use crate::config::DOWNLOAD_POOL_LIMIT;
use crate::error::{Error, Result};
use crate::platform::{HttpClient, ProgressReporter};
use crate::types::Auth;
use futures::future::LocalBoxFuture;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::BTreeMap;

// ── Downloader ────────────────────────────────────────────────────────────────

/// Configuration and entry point for a chunked, client-side-decrypted file download.
///
/// Build with [`Downloader::new`], then call [`Downloader::run`] to execute the transfer.
///
/// # Example
/// ```rust,ignore
/// let plaintext = Downloader::new(auth, file_id, file_size, chunk_count, decryption_key)
///     .run(&http, &progress)
///     .await?;
/// ```
pub struct Downloader {
    auth: Auth,
    file_id: String,
    file_size: u64,
    chunk_count: u64,
    decryption_key: Vec<u8>,
}

impl Downloader {
    /// Create a new downloader with all required parameters.
    pub fn new(
        auth: Auth,
        file_id: impl Into<String>,
        file_size: u64,
        chunk_count: u64,
        decryption_key: Vec<u8>,
    ) -> Self {
        Self {
            auth,
            file_id: file_id.into(),
            file_size,
            chunk_count,
            decryption_key,
        }
    }

    /// Execute the download.
    ///
    /// Fetches all chunks concurrently using a sliding window of up to
    /// [`DOWNLOAD_POOL_LIMIT`] simultaneous requests, decrypts each chunk, and
    /// reassembles them in order.  Returns the complete plaintext as a single
    /// contiguous buffer.
    ///
    /// Progress and cooperative cancellation are handled via `progress`.
    pub async fn run(
        &self,
        http: &dyn HttpClient,
        progress: &dyn ProgressReporter,
    ) -> Result<Vec<u8>> {
        download_file(
            http,
            progress,
            &self.auth,
            &self.file_id,
            self.file_size,
            self.chunk_count,
            &self.decryption_key,
        )
        .await
    }
}

// ── Core download pipeline ────────────────────────────────────────────────────

/// Download and decrypt all chunks of a file, returning the reassembled plaintext.
///
/// Uses a sliding window of [`DOWNLOAD_POOL_LIMIT`] concurrent chunk downloads backed by
/// [`FuturesUnordered`].  Unlike a fixed-batch approach, a new chunk download is dispatched
/// immediately whenever any in-flight download completes, keeping the window continuously full
/// and eliminating the stall that occurs when waiting for an entire batch to finish.
///
/// Chunks may complete out of order; a [`BTreeMap`] buffers completed chunks and drains them
/// in sequence, so the returned buffer is always correctly ordered.
///
/// This free function is the backward-compatible entry point used by tests.
/// New code should prefer the [`Downloader`] builder API.
pub async fn download_file(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &[u8],
) -> Result<Vec<u8>> {
    run_download_pipeline(http, progress, auth, file_id, file_size, chunk_count, decryption_key)
        .await
}

/// Sliding-window download pipeline.
///
/// Maintains exactly [`DOWNLOAD_POOL_LIMIT`] concurrent downloads at all times.
/// Completed (but out-of-order) chunks are buffered in a [`BTreeMap`] and emitted
/// sequentially as their predecessors arrive.
async fn run_download_pipeline<'a>(
    http: &'a dyn HttpClient,
    progress: &'a dyn ProgressReporter,
    auth: &'a Auth,
    file_id: &'a str,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &'a [u8],
) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(file_size as usize);
    let mut in_flight: FuturesUnordered<LocalBoxFuture<'a, (u64, Result<Vec<u8>>)>> =
        FuturesUnordered::new();
    // Holds chunks that have finished downloading but whose predecessor hasn't been emitted yet.
    let mut pending: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    let mut next_to_dispatch: u64 = 0;
    let mut next_to_emit: u64 = 0;
    let mut bytes_downloaded: u64 = 0;

    loop {
        // Fill the sliding window up to DOWNLOAD_POOL_LIMIT concurrent downloads.
        while in_flight.len() < DOWNLOAD_POOL_LIMIT && next_to_dispatch < chunk_count {
            let chunk = next_to_dispatch;
            next_to_dispatch += 1;
            in_flight.push(Box::pin(fetch_and_decrypt(
                http,
                auth,
                file_id,
                chunk,
                decryption_key,
            )));
        }

        if in_flight.is_empty() {
            break;
        }

        // Cooperative cancellation check before each await point.
        if progress.is_cancelled(file_id) {
            return Err(Error::Cancelled);
        }

        // Wait for whichever download finishes first.
        let (chunk_idx, chunk_result) = in_flight.next().await.expect("non-empty");
        pending.insert(chunk_idx, chunk_result?);

        // Drain all consecutively-ready chunks in order.
        while let Some(data) = pending.remove(&next_to_emit) {
            bytes_downloaded += data.len() as u64;
            progress.on_chunk_downloaded(file_id, bytes_downloaded, file_size);
            result.extend_from_slice(&data);
            next_to_emit += 1;
        }
    }

    progress.on_complete(file_id);
    Ok(result)
}

/// Fetch a single encrypted chunk and decrypt it.
///
/// Returns `(chunk_index, result)` so that callers using [`FuturesUnordered`] can
/// associate the result with its position in the file without relying on completion order.
async fn fetch_and_decrypt<'a>(
    http: &'a dyn HttpClient,
    auth: &'a Auth,
    file_id: &'a str,
    chunk: u64,
    decryption_key: &'a [u8],
) -> (u64, Result<Vec<u8>>) {
    let result = async {
        let encrypted = http.download_chunk(auth, file_id, chunk).await?;
        let plaintext =
            cryptfns::aes::decrypt(decryption_key.to_vec(), encrypted).map_err(Error::from)?;
        Ok(plaintext)
    }
    .await;
    (chunk, result)
}
