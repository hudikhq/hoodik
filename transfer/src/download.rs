use crate::config::DOWNLOAD_POOL_LIMIT;
use crate::error::{Error, Result};
use crate::platform::{HttpClient, ProgressReporter};
use crate::types::{Auth, DownloadSource};
use futures::future::LocalBoxFuture;
use std::cell::RefCell;
use std::str::FromStr;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{BTreeMap, HashMap};

/// Minimum growth in received bytes between two progress emissions.
///
/// Streaming reads arrive every few dozen kilobytes; reporting each one
/// would flood the host callback (a JSON `postMessage` per event on web).
/// A quarter megabyte keeps the bar visibly fluid while bounding a
/// multi-gigabyte download to a few thousand events.
const BYTE_PROGRESS_EMIT_DELTA: u64 = 256 * 1024;

/// Cumulative received-byte accounting across concurrently downloading chunks.
///
/// Progress used to be reported only when a chunk was emitted *in order*, so
/// with a full window of in-flight chunks on a slow link the bar sat at zero
/// while megabytes were arriving. The tally tracks bytes per chunk as they
/// stream in — from the first read — and sums them, so the total moves the
/// moment any request receives data.
///
/// Keyed per chunk rather than kept as one running counter so a restarted
/// request simply overwrites its own entry instead of double-counting.
pub(crate) struct ByteTally {
    per_chunk: HashMap<u64, u64>,
    last_emitted: u64,
}

impl ByteTally {
    pub(crate) fn new() -> Self {
        Self {
            per_chunk: HashMap::new(),
            last_emitted: 0,
        }
    }

    /// Record that `bytes` of `chunk` have been received so far and return
    /// the cumulative total to report, or `None` while the growth since the
    /// last emission is below [`BYTE_PROGRESS_EMIT_DELTA`].
    ///
    /// The total is capped at `total_bytes - 1`: received bytes are
    /// ciphertext measured against a plaintext total, and the final exact
    /// figure belongs to the caller once the pipeline actually finishes —
    /// the bar must never claim completion early.
    pub(crate) fn record(&mut self, chunk: u64, bytes: u64, total_bytes: u64) -> Option<u64> {
        self.per_chunk.insert(chunk, bytes);

        let total: u64 = self.per_chunk.values().sum();
        let capped = total.min(total_bytes.saturating_sub(1));

        if capped < self.last_emitted + BYTE_PROGRESS_EMIT_DELTA {
            return None;
        }

        self.last_emitted = capped;
        Some(capped)
    }
}

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
    download_file_from(
        http,
        progress,
        auth,
        DownloadSource::Storage(file_id),
        file_size,
        chunk_count,
        decryption_key,
        cipher,
    )
    .await
}

/// Like [`download_file`], for any [`DownloadSource`] — public share links
/// download the same chunks through their own anonymous route.
#[allow(clippy::too_many_arguments)]
pub async fn download_file_from(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    source: DownloadSource<'_>,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &[u8],
    cipher: &str,
) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(file_size as usize);
    run_download_pipeline(
        http,
        progress,
        auth,
        source,
        file_size,
        chunk_count,
        decryption_key,
        cipher,
        &mut |chunk| result.extend_from_slice(&chunk),
    )
    .await?;
    Ok(result)
}

/// Like [`download_file`], but hands each decrypted chunk to `emit` in
/// file order instead of assembling one contiguous buffer.
///
/// This is what keeps huge files inside wasm32's 4 GB linear memory: the
/// pipeline's peak footprint becomes the in-flight window plus one chunk,
/// independent of file size, and the caller streams the plaintext into
/// storage the platform manages (blob parts on web, a file on disk).
#[allow(clippy::too_many_arguments)]
pub async fn download_file_streaming(
    http: &dyn HttpClient,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    source: DownloadSource<'_>,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &[u8],
    cipher: &str,
    emit: &mut dyn FnMut(Vec<u8>),
) -> Result<()> {
    run_download_pipeline(
        http,
        progress,
        auth,
        source,
        file_size,
        chunk_count,
        decryption_key,
        cipher,
        emit,
    )
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
    source: DownloadSource<'a>,
    file_size: u64,
    chunk_count: u64,
    decryption_key: &'a [u8],
    cipher: &'a str,
    emit: &mut dyn FnMut(Vec<u8>),
) -> Result<()> {
    let mut bytes_emitted: u64 = 0;
    // Declared before `in_flight` so the closures borrowing it outlive the
    // futures they're captured by.
    let tally = RefCell::new(ByteTally::new());
    let mut in_flight: FuturesUnordered<LocalBoxFuture<'_, (u64, Result<Vec<u8>>)>> =
        FuturesUnordered::new();
    // Holds chunks that have finished downloading but whose predecessor hasn't been emitted yet.
    let mut pending: BTreeMap<u64, Vec<u8>> = BTreeMap::new();
    let mut next_to_dispatch: u64 = 0;
    let mut next_to_emit: u64 = 0;

    loop {
        // Fill the sliding window up to DOWNLOAD_POOL_LIMIT concurrent downloads.
        while in_flight.len() < DOWNLOAD_POOL_LIMIT && next_to_dispatch < chunk_count {
            let chunk = next_to_dispatch;
            next_to_dispatch += 1;
            let tally = &tally;
            in_flight.push(Box::pin(fetch_and_decrypt(
                http,
                auth,
                source,
                chunk,
                decryption_key,
                cipher,
                Box::new(move |bytes| {
                    if let Some(total) = tally.borrow_mut().record(chunk, bytes, file_size) {
                        progress.on_chunk_downloaded(source.id(), total, file_size);
                    }
                }),
            )));
        }

        if in_flight.is_empty() {
            break;
        }

        // Cooperative cancellation check before each await point.
        if progress.is_cancelled(source.id()) {
            return Err(Error::Cancelled);
        }

        // Wait for whichever download finishes first.
        let (chunk_idx, chunk_result) = in_flight.next().await.expect("non-empty");
        pending.insert(chunk_idx, chunk_result?);

        // Drain all consecutively-ready chunks in order.
        while let Some(data) = pending.remove(&next_to_emit) {
            bytes_emitted += data.len() as u64;
            emit(data);
            next_to_emit += 1;
        }
    }

    // The exact figure the streaming events deliberately stop short of:
    // every plaintext byte has been handed over, so the bar may reach the end.
    progress.on_chunk_downloaded(source.id(), bytes_emitted, file_size);
    progress.on_complete(source.id());
    Ok(())
}

/// Fetch a single encrypted chunk and decrypt it.
///
/// Returns `(chunk_index, result)` so that callers using [`FuturesUnordered`] can
/// associate the result with its position in the file without relying on completion order.
pub(crate) async fn fetch_and_decrypt<'a>(
    http: &'a dyn HttpClient,
    auth: &'a Auth,
    source: DownloadSource<'a>,
    chunk: u64,
    decryption_key: &'a [u8],
    cipher: &'a str,
    on_bytes: Box<dyn Fn(u64) + 'a>,
) -> (u64, Result<Vec<u8>>) {
    let result = async {
        let encrypted = http.download_chunk(auth, source, chunk, on_bytes).await?;
        let plaintext = cryptfns::cipher::Cipher::from_str(cipher)
            .map_err(Error::from)?
            .decrypt_chunk(decryption_key, chunk, encrypted)
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
#[allow(clippy::too_many_arguments)]
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
    let tally = RefCell::new(ByteTally::new());
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
            let tally = &tally;
            in_flight.push(Box::pin(fetch_and_save(
                http,
                auth,
                DownloadSource::Storage(file_id),
                chunk,
                output_dir,
                Box::new(move |bytes| {
                    if let Some(total) = tally.borrow_mut().record(chunk, bytes, file_size) {
                        progress.on_chunk_downloaded(file_id, total, file_size);
                    }
                }),
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
    }

    progress.on_chunk_downloaded(file_id, bytes_downloaded, file_size);
    progress.on_complete(file_id);
    Ok(())
}

/// Fetch a single encrypted chunk and write it to disk without decrypting.
#[cfg(any(feature = "native", feature = "mobile"))]
async fn fetch_and_save<'a>(
    http: &'a dyn HttpClient,
    auth: &'a Auth,
    source: DownloadSource<'a>,
    chunk: u64,
    output_dir: &'a str,
    on_bytes: Box<dyn Fn(u64) + 'a>,
) -> (u64, Result<usize>) {
    let result = async {
        let encrypted = http.download_chunk(auth, source, chunk, on_bytes).await?;
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

    // The tar body streams as one response; the tally treats it as a
    // single chunk so the bar moves while the archive arrives instead of
    // jumping when it lands.
    let tally = RefCell::new(ByteTally::new());
    let tar_data = http
        .download_all_chunks(
            auth,
            file_id,
            Box::new(|bytes| {
                if let Some(total) = tally.borrow_mut().record(0, bytes, file_size) {
                    progress.on_chunk_downloaded(file_id, total, file_size);
                }
            }),
        )
        .await?;
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
            .decrypt_chunk(decryption_key, i, encrypted)
            .map_err(Error::from)?;

        file.write_all(&plaintext)
            .await
            .map_err(|e| Error::Io(e.to_string()))?;
    }

    file.flush().await.map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}
