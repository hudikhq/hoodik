use crate::checksum;
use crate::config::{
    CHUNK_SIZE_BYTES, CONCURRENT_UPLOADS_IN_FLIGHT, ENCRYPTED_CHUNKS_BUFFER_LIMIT,
    MAX_UPLOAD_RETRIES, UploadHashOptions,
};
use crate::error::{Error, HttpError, Result};
use crate::platform::{DataSource, HttpClient, ProgressReporter};
use crate::types::{Auth, FileHashes};
use crate::upload_trace;
use digest::Digest;
use futures::future::LocalBoxFuture;
use std::str::FromStr;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashSet, VecDeque};

/// Called for every plaintext chunk in order when SHA-256 (and optional other hashes) are
/// offloaded to a separate execution context (e.g. a dedicated Web Worker).
///
/// The host is responsible for feeding the received chunks into its own digest and calling
/// `PUT .../hashes` after the upload completes.  When this hook is provided together with
/// `UploadHashOptions { inline_sha256: false }`, the Rust upload pipeline deliberately skips
/// all inline hashing and calls this hook instead, enabling fully parallel hash computation.
pub trait PlaintextChunkHook {
    fn on_plaintext_chunk(&self, data: &[u8]);
}

/// Configuration and entry point for a chunked, client-side-encrypted file upload.
///
/// Build with [`Uploader::new`], optionally customise with the builder methods, then call
/// [`Uploader::run`] to execute the transfer.
///
/// # Example
/// ```rust,ignore
/// let hashes = Uploader::new(auth, file_id, encryption_key)
///     .with_already_uploaded(&[0, 2])      // resume: skip chunks 0 and 2
///     .with_hash_options(UploadHashOptions::default())
///     .run(&http, &source, &progress, None)
///     .await?;
/// ```
pub struct Uploader {
    auth: Auth,
    file_id: String,
    encryption_key: Vec<u8>,
    already_uploaded: HashSet<u64>,
    hash_options: UploadHashOptions,
    cipher: String,
}

impl Uploader {
    /// Create a new uploader.  Defaults to all optional hashes enabled and inline SHA-256.
    pub fn new(auth: Auth, file_id: impl Into<String>, encryption_key: Vec<u8>) -> Self {
        Self {
            auth,
            file_id: file_id.into(),
            encryption_key,
            already_uploaded: HashSet::new(),
            hash_options: UploadHashOptions::default(),
            cipher: cryptfns::cipher::DEFAULT.to_string(),
        }
    }

    /// Mark chunks that have already been stored on the server (for resume support).
    ///
    /// These chunks are still read from `source` and hashed (so the final digest is correct),
    /// but they are not re-encrypted or re-uploaded.
    pub fn with_already_uploaded(mut self, chunks: &[u64]) -> Self {
        self.already_uploaded = chunks.iter().copied().collect();
        self
    }

    /// Override which content hashes are computed during the upload.
    ///
    /// Use [`UploadHashOptions::from_disable_mask`] to build this from the bitmask constants
    /// exported by the WASM API (e.g. `transferHashOffloadSha256 | transferHashDisableMd5`).
    pub fn with_hash_options(mut self, opts: UploadHashOptions) -> Self {
        self.hash_options = opts;
        self
    }

    /// Set the cipher to use for chunk encryption (e.g. `"ascon128a"`, `"chacha20poly1305"`).
    /// Defaults to [`cryptfns::cipher::DEFAULT`] when not called.
    pub fn with_cipher(mut self, cipher: impl Into<String>) -> Self {
        self.cipher = cipher.into();
        self
    }

    /// Execute the upload.
    ///
    /// Streams chunks from `source`, hashes them (inline or via `plaintext_hook`), encrypts
    /// each chunk, and uploads them concurrently via `http`.  Progress and cancellation are
    /// reported/checked through `progress`.
    ///
    /// When `hash_options.inline_sha256 = false` (offload mode), `plaintext_hook` **must** be
    /// `Some`; every plaintext chunk is forwarded to the hook in order so the host can compute
    /// the file hash externally (e.g. in a dedicated Web Worker).  The returned [`FileHashes`]
    /// will contain an empty `sha256` string in that case — the host is responsible for
    /// submitting the real hash via `PUT .../hashes` after this call returns.
    pub async fn run(
        &self,
        http: &dyn HttpClient,
        source: &dyn DataSource,
        progress: &dyn ProgressReporter,
        plaintext_hook: Option<&dyn PlaintextChunkHook>,
    ) -> Result<FileHashes> {
        upload_file(
            http,
            source,
            progress,
            &self.auth,
            &self.file_id,
            &self.encryption_key,
            &self.already_uploaded.iter().copied().collect::<Vec<_>>(),
            self.hash_options,
            plaintext_hook,
            &self.cipher,
        )
        .await
    }
}

/// Incremental hash state accumulated over all plaintext chunks during upload.
///
/// Only handles the **inline** hashing path.  When `inline_sha256 = false` (offload mode),
/// [`HashState::update_inline`] is a no-op and [`HashState::finalize`] returns empty strings —
/// the host computes hashes externally via [`PlaintextChunkHook`].
struct HashState {
    h_sha256: sha2::Sha256,
    h_md5: Option<md5::Md5>,
    h_sha1: Option<sha1::Sha1>,
    h_blake2b: Option<blake2::Blake2b512>,
    /// When false the upload WASM does not compute hashes; the host handles it externally.
    inline_sha256: bool,
}

impl HashState {
    fn new(opts: &UploadHashOptions) -> Self {
        Self {
            h_sha256: sha2::Sha256::new(),
            h_md5: opts.md5.then(md5::Md5::new),
            h_sha1: opts.sha1.then(sha1::Sha1::new),
            h_blake2b: opts.blake2b.then(blake2::Blake2b512::new),
            inline_sha256: opts.inline_sha256,
        }
    }

    /// Feed a plaintext chunk into all active digest algorithms.
    ///
    /// No-op when `inline_sha256 = false` (offload mode) — the host is hashing externally.
    fn update_inline(&mut self, data: &[u8]) {
        if !self.inline_sha256 {
            return;
        }
        self.h_sha256.update(data);
        if let Some(ref mut h) = self.h_md5 {
            h.update(data);
        }
        if let Some(ref mut h) = self.h_sha1 {
            h.update(data);
        }
        if let Some(ref mut h) = self.h_blake2b {
            h.update(data);
        }
    }

    /// Finalise all digests and return the computed [`FileHashes`].
    ///
    /// In offload mode the `sha256` field (and optional hash fields) will be empty strings —
    /// the caller should not submit these to the server; the host will do so after obtaining
    /// the real hashes from the external hash worker.
    fn finalize(self) -> FileHashes {
        if self.inline_sha256 {
            FileHashes {
                md5: self.h_md5.map(|h| hex::encode(h.finalize())),
                sha1: self.h_sha1.map(|h| hex::encode(h.finalize())),
                sha256: hex::encode(self.h_sha256.finalize()),
                blake2b: self.h_blake2b.map(|h| hex::encode(h.finalize())),
            }
        } else {
            // Offload mode: hashes are computed externally by the host (e.g. hash-worker).
            FileHashes {
                md5: None,
                sha1: None,
                sha256: String::new(),
                blake2b: None,
            }
        }
    }
}

/// Run a full chunked, encrypted upload of a file.
///
/// Reads and hashes chunks **in order** (streaming digests). Chunks are then encrypted and
/// queued for upload with two independent limits:
/// - max active HTTP uploads: [`CONCURRENT_UPLOADS_IN_FLIGHT`]
/// - max encrypted chunks buffered in memory (uploading + waiting): [`ENCRYPTED_CHUNKS_BUFFER_LIMIT`]
///   This keeps encryption flowing while bounding memory usage.
///
/// Use `hash_options` to skip optional algorithms; SHA-256 is always computed unless offloaded.
/// Chunks in `already_uploaded` are read for hashing but skip encryption and upload.
///
/// This free function is the backward-compatible entry point used by tests.
/// New code should prefer the [`Uploader`] builder API.
#[allow(clippy::too_many_arguments)]
pub async fn upload_file(
    http: &dyn HttpClient,
    source: &dyn DataSource,
    progress: &dyn ProgressReporter,
    auth: &Auth,
    file_id: &str,
    encryption_key: &[u8],
    already_uploaded: &[u64],
    hash_options: UploadHashOptions,
    plaintext_hook: Option<&dyn PlaintextChunkHook>,
    cipher: &str,
) -> Result<FileHashes> {
    // When inline_sha256 = false and no hook is provided, all hash computation is skipped.
    // The caller is responsible for computing and submitting hashes via another mechanism.

    let total_size = source.total_size();
    let total_chunks = compute_chunk_count(total_size);
    let already_uploaded_set: HashSet<u64> = already_uploaded.iter().copied().collect();

    let mut hash_state = HashState::new(&hash_options);

    upload_trace::hash_log(&format!(
        "[transfer:hash] file_id={} inline_sha256={} md5={} sha1={} blake2b={} plaintext_hook={}",
        short_id(file_id),
        hash_options.inline_sha256,
        hash_options.md5,
        hash_options.sha1,
        hash_options.blake2b,
        plaintext_hook.is_some()
    ));

    // Run the main upload pipeline (reads, hashes, encrypts, uploads concurrently).
    run_upload_pipeline(
        http,
        source,
        progress,
        auth,
        file_id,
        encryption_key,
        &already_uploaded_set,
        total_size,
        total_chunks,
        &mut hash_state,
        plaintext_hook,
        cipher,
    )
    .await?;

    let hashes = hash_state.finalize();

    upload_trace::hash_log(&format!(
        "[transfer:hash] file_id={} inline_sha256={} sha256_len={} md5={} sha1={} blake2b={}",
        short_id(file_id),
        hash_options.inline_sha256,
        hashes.sha256.len(),
        hashes.md5.is_some(),
        hashes.sha1.is_some(),
        hashes.blake2b.is_some(),
    ));

    // In inline mode the Rust side has all the hashes and submits them to the server.
    // In offload mode the host (e.g. sw.ts) is responsible for submitting hashes after the
    // hash-worker finalises — so we skip the PUT here.
    if hash_options.inline_sha256 {
        if let Err(e) = http.update_hashes(auth, file_id, &hashes).await {
            progress.on_error(file_id, &format!("Failed to update hashes: {e}"));
        }
    }
    progress.on_complete(file_id);

    Ok(hashes)
}

/// Inner upload pipeline: streams chunks, hashes them, encrypts, and manages the concurrent
/// HTTP upload queue.
///
/// Operates in two concurrent stages connected by a bounded in-memory queue:
/// 1. **Producer** (this loop): reads plaintext → hashes → encrypts → pushes to `encrypted_waiting`
/// 2. **Consumer** ([`pump_uploads`]): pops from `encrypted_waiting` → dispatches HTTP uploads
///
/// Two limits prevent unbounded memory growth:
/// - [`ENCRYPTED_CHUNKS_BUFFER_LIMIT`]: total encrypted chunks in memory (waiting + in-flight)
/// - [`CONCURRENT_UPLOADS_IN_FLIGHT`]: HTTP requests in flight simultaneously
#[allow(clippy::too_many_arguments)]
async fn run_upload_pipeline<'a>(
    http: &'a dyn HttpClient,
    source: &'a dyn DataSource,
    progress: &'a dyn ProgressReporter,
    auth: &'a Auth,
    file_id: &'a str,
    encryption_key: &'a [u8],
    already_uploaded: &'a HashSet<u64>,
    total_size: u64,
    total_chunks: u64,
    hash_state: &mut HashState,
    plaintext_hook: Option<&'a dyn PlaintextChunkHook>,
    cipher: &str,
) -> Result<()> {
    let mut in_flight: FuturesUnordered<LocalBoxFuture<'a, Result<u64>>> = FuturesUnordered::new();
    let mut encrypted_waiting = VecDeque::<EncryptedChunk>::new();
    let in_flight_cap = CONCURRENT_UPLOADS_IN_FLIGHT;
    // Start HTTP uploads as soon as at least one encrypted chunk is buffered —
    // this prevents uploads from waiting until the full buffer is filled.
    let preparing_threshold = 1;
    let mut preparing = true;
    let t_upload_start = upload_trace::now_ms();
    let mut plaintext_hook_calls: u64 = 0;
    let mut first_hook_chunk_len: u64 = 0;
    let mut hooked_once = false;

    upload_trace::log(&format!(
        "[transfer:upload] start file_id={} total_chunks={} cap={}",
        short_id(file_id),
        total_chunks,
        in_flight_cap
    ));

    for chunk in 0..total_chunks {
        // Cooperative cancellation check — polled every 8 chunks to avoid call overhead.
        if chunk % 8 == 0 && progress.is_cancelled(file_id) {
            return Err(Error::Cancelled);
        }

        // Memory backpressure: wait for an upload slot when the buffer is full.
        while in_flight.len() + encrypted_waiting.len() >= ENCRYPTED_CHUNKS_BUFFER_LIMIT {
            preparing = false;
            pump_uploads(
                &mut in_flight,
                &mut encrypted_waiting,
                in_flight_cap,
                http,
                auth,
                file_id,
                total_chunks,
                progress,
            );
            let t_wait = upload_trace::now_ms();
            upload_trace::log(&format!(
                "[transfer:upload] memory backpressure file_id={} chunk={} uploading={} buffered={} (waiting)",
                short_id(file_id),
                chunk,
                in_flight.len(),
                encrypted_waiting.len()
            ));
            match in_flight.next().await {
                Some(Ok(done_chunk)) => {
                    let wait_ms = upload_trace::now_ms() - t_wait;
                    upload_trace::log(&format!(
                        "[transfer:upload] upload done file_id={} chunk={} after {:.1}ms (still on chunk {})",
                        short_id(file_id),
                        done_chunk,
                        wait_ms,
                        chunk
                    ));
                }
                Some(Err(e)) => {
                    progress.on_error(file_id, &format!("{e}"));
                    return Err(e);
                }
                None => break,
            }
        }

        // Read the next plaintext chunk from the data source.
        let offset = chunk * CHUNK_SIZE_BYTES;
        let length = if chunk == total_chunks - 1 {
            total_size - offset
        } else {
            CHUNK_SIZE_BYTES
        };

        let t_read = upload_trace::now_ms();
        let plaintext = source.read_chunk(offset, length).await?;
        let read_ms = upload_trace::now_ms() - t_read;

        // Hash the plaintext chunk.
        // Two mutually exclusive paths based on whether hashing is done inline or offloaded:
        //   Inline  → update running digests inside this WASM thread.
        //   Offload → forward raw bytes to the host (e.g. dedicated hash Web Worker) via hook.
        let t_hash = upload_trace::now_ms();
        hash_state.update_inline(&plaintext);
        if !hash_state.inline_sha256 {
            if let Some(hook) = plaintext_hook {
                hook.on_plaintext_chunk(&plaintext);
                plaintext_hook_calls += 1;
                if !hooked_once {
                    hooked_once = true;
                    first_hook_chunk_len = plaintext.len() as u64;
                }
            }
        }
        let hash_ms = upload_trace::now_ms() - t_hash;

        // Encrypt and queue the chunk (skip if already stored on the server).
        if !already_uploaded.contains(&chunk) {
            let t_enc = upload_trace::now_ms();
            let encrypted = cryptfns::cipher::Cipher::from_str(cipher)
                .map_err(Error::from)?
                .encrypt(encryption_key.to_vec(), plaintext)
                .map_err(Error::from)?;
            let enc_ms = upload_trace::now_ms() - t_enc;
            if encrypted.is_empty() {
                return Err(Error::Io(format!(
                    "Encryption produced empty output for chunk {chunk}"
                )));
            }

            let crc = checksum::crc16(&encrypted);
            encrypted_waiting.push_back(EncryptedChunk {
                chunk,
                encrypted,
                crc,
            });
            upload_trace::log(&format!(
                "[transfer:upload] chunk {} encrypted in {:.1}ms (buffered={})",
                chunk,
                enc_ms,
                encrypted_waiting.len()
            ));

            // Begin dispatching uploads once we have at least `preparing_threshold` buffered.
            if !preparing || encrypted_waiting.len() >= preparing_threshold {
                preparing = false;
                pump_uploads(
                    &mut in_flight,
                    &mut encrypted_waiting,
                    in_flight_cap,
                    http,
                    auth,
                    file_id,
                    total_chunks,
                    progress,
                );
            }
        }

        if !already_uploaded.contains(&chunk) {
            upload_trace::log(&format!(
                "[transfer:upload] chunk {} read {:.1}ms hash {:.1}ms (uploading={} buffered={} elapsed {:.0}ms)",
                chunk,
                read_ms,
                hash_ms,
                in_flight.len(),
                encrypted_waiting.len(),
                upload_trace::now_ms() - t_upload_start
            ));
        } else {
            upload_trace::log(&format!(
                "[transfer:upload] chunk {} skip upload (already stored) read {:.1}ms hash {:.1}ms",
                chunk, read_ms, hash_ms
            ));
        }
    }

    // Dispatch any remaining buffered chunks that weren't dispatched in the loop above.
    pump_uploads(
        &mut in_flight,
        &mut encrypted_waiting,
        in_flight_cap,
        http,
        auth,
        file_id,
        total_chunks,
        progress,
    );
    upload_trace::log(&format!(
        "[transfer:upload] main loop done file_id={} draining uploading={} buffered={}",
        short_id(file_id),
        in_flight.len(),
        encrypted_waiting.len()
    ));

    // Drain: wait for all in-flight uploads and dispatch any still-waiting chunks.
    let t_drain = upload_trace::now_ms();
    while !encrypted_waiting.is_empty() || !in_flight.is_empty() {
        pump_uploads(
            &mut in_flight,
            &mut encrypted_waiting,
            in_flight_cap,
            http,
            auth,
            file_id,
            total_chunks,
            progress,
        );
        let Some(result) = in_flight.next().await else {
            break;
        };
        match result {
            Ok(_) => {}
            Err(e) => {
                progress.on_error(file_id, &format!("{e}"));
                return Err(e);
            }
        }
    }
    upload_trace::log(&format!(
        "[transfer:upload] drain done file_id={} in {:.1}ms",
        short_id(file_id),
        upload_trace::now_ms() - t_drain
    ));

    upload_trace::hash_log(&format!(
        "[transfer:hash] pipeline done file_id={} offload_hook_calls={} first_hook_chunk_len={}",
        short_id(file_id),
        plaintext_hook_calls,
        first_hook_chunk_len
    ));

    Ok(())
}

/// Move as many chunks as possible from `encrypted_waiting` into `in_flight`,
/// up to `in_flight_cap` concurrent requests.
#[allow(clippy::too_many_arguments)]
fn pump_uploads<'a>(
    in_flight: &mut FuturesUnordered<LocalBoxFuture<'a, Result<u64>>>,
    encrypted_waiting: &mut VecDeque<EncryptedChunk>,
    in_flight_cap: usize,
    http: &'a dyn HttpClient,
    auth: &'a Auth,
    file_id: &'a str,
    total_chunks: u64,
    progress: &'a dyn ProgressReporter,
) {
    while in_flight.len() < in_flight_cap {
        let Some(next) = encrypted_waiting.pop_front() else {
            break;
        };
        upload_trace::log(&format!(
            "[transfer:upload] dispatch chunk {} (uploading={} buffered={})",
            next.chunk,
            in_flight.len() + 1,
            encrypted_waiting.len()
        ));
        in_flight.push(Box::pin(upload_encrypted(
            http,
            auth,
            file_id,
            next.chunk,
            total_chunks,
            next.crc,
            next.encrypted,
            progress,
            0,
        )));
    }
}

struct EncryptedChunk {
    chunk: u64,
    encrypted: Vec<u8>,
    crc: String,
}

/// Upload one encrypted chunk to the server, retrying on CRC16 checksum mismatch.
///
/// A checksum mismatch means the server received corrupted bytes; we retry with the same
/// encrypted data.  After [`MAX_UPLOAD_RETRIES`] retries the error is propagated.
/// A `chunk_already_exists` error is treated as success (idempotent upload).
#[allow(clippy::too_many_arguments)]
async fn upload_encrypted(
    http: &dyn HttpClient,
    auth: &Auth,
    file_id: &str,
    chunk: u64,
    total_chunks: u64,
    crc: String,
    encrypted: Vec<u8>,
    progress: &dyn ProgressReporter,
    attempt: u32,
) -> Result<u64> {
    let t0 = upload_trace::now_ms();

    let t_http = upload_trace::now_ms();
    match http
        .upload_chunk(auth, file_id, chunk, &crc, &encrypted)
        .await
    {
        Ok(resp) => {
            let http_ms = upload_trace::now_ms() - t_http;
            upload_trace::log(&format!(
                "[transfer:upload] chunk {} HTTP ok http {:.1}ms total {:.1}ms",
                chunk,
                http_ms,
                upload_trace::now_ms() - t0
            ));
            let stored = resp.chunks_stored.unwrap_or(0) as u64;
            let is_done = stored == total_chunks;
            progress.on_chunk_uploaded(file_id, chunk, total_chunks, is_done);
            Ok(chunk)
        }
        // Checksum mismatch — server received corrupted bytes; retry the same encrypted data.
        Err(Error::Http(HttpError { validation, .. }))
            if validation
                .as_ref()
                .is_some_and(|v| v.contains_key("checksum"))
                && attempt < MAX_UPLOAD_RETRIES =>
        {
            Box::pin(upload_encrypted(
                http,
                auth,
                file_id,
                chunk,
                total_chunks,
                crc,
                encrypted,
                progress,
                attempt + 1,
            ))
            .await
        }
        // Chunk already stored — treat as success (idempotent).
        Err(Error::Http(HttpError { validation, .. }))
            if validation
                .as_ref()
                .is_some_and(|v| v.get("chunk").is_some_and(|v| v == "chunk_already_exists")) =>
        {
            progress.on_chunk_uploaded(file_id, chunk, total_chunks, false);
            Ok(chunk)
        }
        Err(e) => Err(e),
    }
}

/// Compute the number of chunks required to upload `total_size` bytes.
/// An empty file still requires one (empty) chunk.
pub(crate) fn compute_chunk_count(total_size: u64) -> u64 {
    if total_size == 0 {
        return 1;
    }
    total_size.div_ceil(CHUNK_SIZE_BYTES)
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}
