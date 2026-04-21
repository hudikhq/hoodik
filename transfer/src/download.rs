use crate::config::DOWNLOAD_POOL_LIMIT;
use crate::error::{Error, Result};
use crate::platform::{HttpClient, ProgressReporter};
use crate::types::Auth;
use futures::future::LocalBoxFuture;
use std::str::FromStr;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::BTreeMap;

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
    cipher: String,
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
            cipher: cryptfns::cipher::DEFAULT.to_string(),
        }
    }

    /// Set the cipher to use for chunk decryption (e.g. `"ascon128a"`, `"chacha20poly1305"`).
    /// Defaults to [`cryptfns::cipher::DEFAULT`] when not called.
    pub fn with_cipher(mut self, cipher: impl Into<String>) -> Self {
        self.cipher = cipher.into();
        self
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
            &self.cipher,
        )
        .await
    }
}

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
#[allow(clippy::too_many_arguments)]
pub async fn download_file(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &[u8],
    cipher: &str,
) -> Result<Vec<u8>> {
    run_download_pipeline(http, progress, auth, file_id, file_size, chunk_count, decryption_key, cipher)
        .await
}

/// Sliding-window download pipeline.
///
/// Maintains exactly [`DOWNLOAD_POOL_LIMIT`] concurrent downloads at all times.
/// Completed (but out-of-order) chunks are buffered in a [`BTreeMap`] and emitted
/// sequentially as their predecessors arrive.
#[allow(clippy::too_many_arguments)]
async fn run_download_pipeline<'a>(
    http: &'a dyn HttpClient,
    progress: &'a dyn ProgressReporter,
    auth: &'a Auth,
    file_id: &'a str,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &'a [u8],
    cipher: &'a str,
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
                cipher,
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
    cipher: &'a str,
) -> (u64, Result<Vec<u8>>) {
    let result = async {
        let encrypted = http.download_chunk(auth, file_id, chunk).await?;
        let plaintext = cryptfns::cipher::Cipher::from_str(cipher)
            .map_err(Error::from)?
            .decrypt(decryption_key.to_vec(), encrypted)
            .map_err(Error::from)?;
        Ok(plaintext)
    }
    .await;
    (chunk, result)
}

/// Download all chunks to individual files without decrypting.
///
/// Each chunk is written to `{output_dir}/{chunk_index:06}.enc` immediately
/// after fetch.  Uses the same sliding-window concurrency as [`download_file`]
/// but skips decryption entirely, keeping peak memory at ~4 MB per in-flight
/// chunk.
///
/// `already_downloaded` lists chunk indices to skip (for resume support).
#[cfg(any(feature = "native", feature = "mobile"))]
pub async fn download_chunks_to_dir(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    file_size: u64,
    chunk_count: u64,
    output_dir: &str,
    already_downloaded: &[u64],
) -> Result<()> {
    use std::collections::HashSet;

    let skip: HashSet<u64> = already_downloaded.iter().copied().collect();
    let mut in_flight: FuturesUnordered<LocalBoxFuture<'_, (u64, Result<usize>)>> =
        FuturesUnordered::new();
    let mut next_to_dispatch: u64 = 0;
    let mut bytes_downloaded: u64 = 0;

    loop {
        // Fill the sliding window, skipping already-downloaded chunks.
        while in_flight.len() < DOWNLOAD_POOL_LIMIT && next_to_dispatch < chunk_count {
            let chunk = next_to_dispatch;
            next_to_dispatch += 1;
            if skip.contains(&chunk) {
                continue;
            }
            in_flight.push(Box::pin(fetch_and_save(
                http, auth, file_id, chunk, output_dir,
            )));
        }

        if in_flight.is_empty() {
            break;
        }

        if progress.is_cancelled(file_id) {
            return Err(Error::Cancelled);
        }

        let (_chunk_idx, chunk_result) = in_flight.next().await.expect("non-empty");
        let chunk_len = chunk_result?;
        bytes_downloaded += chunk_len as u64;
        progress.on_chunk_downloaded(file_id, bytes_downloaded, file_size);
    }

    progress.on_complete(file_id);
    Ok(())
}

/// Fetch a single encrypted chunk and write it to disk without decrypting.
#[cfg(any(feature = "native", feature = "mobile"))]
async fn fetch_and_save<'a>(
    http: &'a dyn HttpClient,
    auth: &'a Auth,
    file_id: &'a str,
    chunk: u64,
    output_dir: &'a str,
) -> (u64, Result<usize>) {
    let result = async {
        let encrypted = http.download_chunk(auth, file_id, chunk).await?;
        let len = encrypted.len();
        let path = format!("{}/{:06}.enc", output_dir, chunk);
        tokio::fs::write(&path, &encrypted)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;
        Ok(len)
    }
    .await;
    (chunk, result)
}

/// Download all chunks in a single request as a tar archive, then extract
/// each chunk to `{output_dir}/{entry_name}` (e.g. `000000.enc`, `000001.enc`).
///
/// This replaces N individual HTTP requests with one, reducing connection
/// overhead while preserving chunk boundaries for later decryption.
#[cfg(any(feature = "native", feature = "mobile"))]
pub async fn download_chunks_to_dir_bulk(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    file_size: u64,
    output_dir: &str,
) -> Result<()> {
    if progress.is_cancelled(file_id) {
        return Err(Error::Cancelled);
    }

    let tar_data = http.download_all_chunks(auth, file_id).await?;
    let entries = crate::tar::extract_tar(&tar_data)?;

    let mut bytes_written: u64 = 0;
    for entry in &entries {
        let path = format!("{}/{}", output_dir, entry.name);
        tokio::fs::write(&path, &entry.data)
            .await
            .map_err(|e| Error::Io(format!("Failed to write {}: {e}", entry.name)))?;

        bytes_written += entry.data.len() as u64;
        progress.on_chunk_downloaded(file_id, bytes_written, file_size);
    }

    progress.on_complete(file_id);
    Ok(())
}

/// Decrypt previously downloaded encrypted chunks to a single output file.
///
/// Reads chunks sequentially from `{chunks_dir}/{index:06}.enc`, decrypts each
/// with the given key+cipher, and appends to `output_path`.  Peak memory usage
/// is one chunk (~4 MB).
#[cfg(any(feature = "native", feature = "mobile"))]
pub async fn decrypt_chunks_to_file(
    chunks_dir: &str,
    chunk_count: u64,
    decryption_key: &[u8],
    cipher: &str,
    output_path: &str,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let cipher_impl = cryptfns::cipher::Cipher::from_str(cipher).map_err(Error::from)?;
    let mut file = tokio::fs::File::create(output_path)
        .await
        .map_err(|e| Error::Io(e.to_string()))?;

    for i in 0..chunk_count {
        let chunk_path = format!("{}/{:06}.enc", chunks_dir, i);
        let encrypted = tokio::fs::read(&chunk_path)
            .await
            .map_err(|e| Error::Io(format!("chunk {i}: {e}")))?;

        let plaintext = cipher_impl
            .decrypt(decryption_key.to_vec(), encrypted)
            .map_err(Error::from)?;

        file.write_all(&plaintext)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;
    }

    file.flush().await.map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}
