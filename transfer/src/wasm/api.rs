use crate::config::{
    UploadHashOptions, HASH_DISABLE_BLAKE2B, HASH_DISABLE_MD5, HASH_DISABLE_SHA1,
    HASH_OFFLOAD_SHA256,
};
use crate::types::{Auth, ChunkTarget, DownloadSource, FileHashes};
use crate::wasm::http::WasmHttpClient;
use crate::wasm::progress::JsProgressReporter;
use crate::wasm::source::FileSource;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// WASM class for client-side-encrypted file upload.
///
/// Construct with `new TransferUploader(...)`, optionally call setter methods to configure
/// resume support or hash options, then `await uploader.upload(...)`.
///
/// Always call `uploader.free()` when done to release WASM memory.
///
/// # JS / TypeScript example
/// ```ts
/// const uploader = new TransferUploader(fileId, baseUrl, jwtToken, refreshToken, encKey)
/// uploader.set_uploaded_chunks(Array.from(alreadyUploadedChunks))
/// uploader.set_hash_mask(transferHashOffloadSha256() | transferHashDisableMd5())
/// const hashes = await uploader.upload(file, onPlaintextChunk, onProgress, isCancelled)
/// uploader.free()
/// ```
#[wasm_bindgen]
pub struct TransferUploader {
    auth: Auth,
    file_id: String,
    encryption_key: Vec<u8>,
    /// Chunk indices that have already been stored on the server (resume support).
    uploaded_chunks: Vec<u32>,
    /// Bitmask of `HASH_DISABLE_*` and optionally `HASH_OFFLOAD_SHA256` flags.
    hash_mask: u32,
    /// Cipher identifier (e.g. `"ascon128a"`, `"chacha20poly1305"`).
    cipher: String,
    /// Presigned bucket URLs indexed by chunk, when the deployment serves them.
    direct_urls: Option<Vec<String>>,
}

#[wasm_bindgen]
impl TransferUploader {
    /// Create a new uploader.
    ///
    /// - `file_id`: UUID of the file record on the server.
    /// - `base_url`: API base URL (e.g. `"https://app.example.com"`).
    /// - `jwt_token`: Optional Bearer token for authentication.
    /// - `refresh_token`: Optional refresh token sent as `X-Auth-Refresh`.
    /// - `encryption_key`: Raw AES key bytes used to encrypt each chunk.
    #[wasm_bindgen(constructor)]
    pub fn new(
        file_id: String,
        base_url: String,
        jwt_token: Option<String>,
        refresh_token: Option<String>,
        encryption_key: Vec<u8>,
    ) -> TransferUploader {
        TransferUploader {
            auth: Auth {
                base_url,
                jwt_token,
                refresh_token,
                cookie: None,
            },
            file_id,
            encryption_key,
            uploaded_chunks: Vec::new(),
            hash_mask: 0,
            cipher: cryptfns::cipher::DEFAULT.to_string(),
            direct_urls: None,
        }
    }

    /// Set the cipher used to encrypt each chunk.
    /// Accepts `"ascon128a"` (default) or `"chacha20poly1305"`.
    /// Must be called before [`upload`].
    #[wasm_bindgen(js_name = "set_cipher")]
    pub fn set_cipher(&mut self, cipher: String) {
        self.cipher = cipher;
    }

    /// Presigned bucket URLs indexed by chunk, so those chunks are written
    /// straight into storage instead of through this server.
    ///
    /// Obtain them from `POST /api/storage/{id}/upload-urls`, declaring the
    /// sizes [`transferEncryptedChunkSizes`] returns — the server signs each
    /// length into its URL and the bucket refuses a body of any other size.
    /// An index left empty keeps using the relaying route. Call before
    /// [`upload`].
    #[wasm_bindgen(js_name = "set_direct_urls")]
    pub fn set_direct_urls(&mut self, urls: Vec<String>) {
        self.direct_urls = Some(urls);
    }

    /// Set the list of chunk indices already stored on the server.
    ///
    /// These chunks will be re-read and re-hashed (to produce the correct final digest)
    /// but will not be re-encrypted or re-uploaded.  Call before [`upload`].
    #[wasm_bindgen(js_name = "set_uploaded_chunks")]
    pub fn set_uploaded_chunks(&mut self, chunks: Vec<u32>) {
        self.uploaded_chunks = chunks;
    }

    /// Set the hash disable / offload bitmask.
    ///
    /// OR together any of:
    /// - [`transferHashDisableMd5`] — skip MD5
    /// - [`transferHashDisableSha1`] — skip SHA-1
    /// - [`transferHashDisableBlake2b`] — skip BLAKE2b-512
    /// - [`transferHashOffloadSha256`] — do not compute SHA-256 inline; the host must hash
    ///   plaintext chunks via the `on_plaintext_chunk` callback and call `PUT .../hashes`
    ///   after the upload completes.
    #[wasm_bindgen(js_name = "set_hash_mask")]
    pub fn set_hash_mask(&mut self, mask: u32) {
        self.hash_mask = mask;
    }

    /// Upload the file with client-side encryption.
    ///
    /// All configuration fields are cloned before the first `.await` so the future is `'static`.
    ///
    /// - `file`: The browser `File` object to upload.
    /// - `external_hash`: Optional `Promise<string>` that resolves to the SHA-256 hex digest
    ///   computed externally (e.g. by a dedicated hash Web Worker reading the file in parallel).
    ///   When provided, inline SHA-256 is skipped entirely; the WASM awaits the promise after all
    ///   chunks are uploaded and includes the result in the returned hashes.
    ///   Pass `undefined` to compute SHA-256 inline (slower but no external dependency).
    /// - `on_progress`: JS callback called with a JSON string on each chunk event.
    /// - `is_cancelled`: JS function polled with `(fileId: string) => boolean`.
    ///
    /// Returns a JSON object `{ sha256, md5?, sha1?, blake2b? }` with the file hashes.
    pub async fn upload(
        &self,
        file: web_sys::File,
        external_hash: Option<js_sys::Promise>,
        on_progress: js_sys::Function,
        is_cancelled: js_sys::Function,
    ) -> Result<JsValue, JsValue> {
        let auth = self.auth.clone();
        let file_id = self.file_id.clone();
        let encryption_key = self.encryption_key.clone();
        let cipher = self.cipher.clone();
        let direct_urls = self.direct_urls.clone();
        let already: Vec<u64> = self.uploaded_chunks.iter().map(|&c| c as u64).collect();

        // When an external hash promise is supplied, skip all inline hashing — the caller
        // is computing SHA-256 in a parallel worker that reads the file independently.
        let mut hash_options = UploadHashOptions::from_disable_mask(self.hash_mask);
        if external_hash.is_some() {
            hash_options.inline_sha256 = false;
        }

        let http = WasmHttpClient::new();
        let source = FileSource::new(file);
        let reporter = JsProgressReporter::new(on_progress, is_cancelled);

        let hashes = crate::upload::upload_file(
            &http,
            &source,
            &reporter,
            &auth,
            &file_id,
            &encryption_key,
            &already,
            hash_options,
            None,
            &cipher,
            direct_urls.as_deref(),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;

        // If the caller provided an external hash promise, await it now (the upload is done,
        // so the hash worker has had the full upload duration to finish reading the file).
        // Return the resolved sha256 so the caller can persist it to the server.
        if let Some(promise) = external_hash {
            let sha256_js = JsFuture::from(promise)
                .await
                .map_err(|e| JsValue::from_str(&format!("external hash promise rejected: {e:?}")))?;
            let sha256 = sha256_js.as_string().unwrap_or_default();
            let final_hashes = FileHashes { sha256, ..hashes };
            return to_value(&final_hashes).map_err(|e| JsValue::from_str(&format!("{e}")));
        }

        to_value(&hashes).map_err(|e| JsValue::from_str(&format!("{e}")))
    }
}

/// WASM class for client-side-decrypted file download.
///
/// Construct with `new TransferDownloader(...)`, then `await downloader.download(...)`.
/// Always call `downloader.free()` when done.
///
/// # JS / TypeScript example
/// ```ts
/// const downloader = new TransferDownloader(
///   fileId, fileSize, chunkCount, baseUrl, jwtToken, refreshToken, decryptionKey
/// )
/// const bytes = await downloader.download(onProgress, isCancelled)
/// downloader.free()
/// ```
#[wasm_bindgen]
pub struct TransferDownloader {
    /// File id, or link id when [`Self::public_link`] is set.
    file_id: String,
    /// File size in bytes (stored as u64; constructor accepts f64 for JS Number compatibility).
    file_size: u64,
    chunk_count: u64,
    auth: Auth,
    decryption_key: Vec<u8>,
    /// Cipher identifier (e.g. `"ascon128a"`, `"chacha20poly1305"`).
    cipher: String,
    /// Chunks come from the anonymous public-link route instead of storage.
    public_link: bool,
    /// Presigned bucket URLs by chunk index, when the host fetched a manifest.
    direct_urls: Option<Vec<String>>,
}

fn source_of(id: &str, public_link: bool) -> DownloadSource<'_> {
    if public_link {
        DownloadSource::PublicLink(id)
    } else {
        DownloadSource::Storage(id)
    }
}

#[wasm_bindgen]
impl TransferDownloader {
    /// Create a new downloader.
    ///
    /// - `file_id`: UUID of the file record on the server.
    /// - `file_size`: Total plaintext size in bytes (JS `Number` / `f64`).
    /// - `chunk_count`: Total number of encrypted chunks stored on the server.
    /// - `base_url`: API base URL.
    /// - `jwt_token`: Optional Bearer token.
    /// - `refresh_token`: Optional refresh token.
    /// - `decryption_key`: Raw AES key bytes used to decrypt each chunk.
    #[wasm_bindgen(constructor)]
    pub fn new(
        file_id: String,
        file_size: f64,
        chunk_count: u32,
        base_url: String,
        jwt_token: Option<String>,
        refresh_token: Option<String>,
        decryption_key: Vec<u8>,
    ) -> TransferDownloader {
        TransferDownloader {
            file_id,
            file_size: file_size as u64,
            chunk_count: chunk_count as u64,
            auth: Auth {
                base_url,
                jwt_token,
                refresh_token,
                cookie: None,
            },
            decryption_key,
            cipher: cryptfns::cipher::DEFAULT.to_string(),
            public_link: false,
            direct_urls: None,
        }
    }

    /// Create a downloader for a public share link.
    ///
    /// Chunks come from `POST /api/links/{link_id}` — anonymous by design, so
    /// there are no tokens here. The key is whatever the caller unwrapped from
    /// the link metadata using the fragment key; the server only ever streams
    /// ciphertext.
    #[wasm_bindgen(js_name = "forPublicLink")]
    pub fn for_public_link(
        link_id: String,
        file_size: f64,
        chunk_count: u32,
        base_url: String,
        decryption_key: Vec<u8>,
    ) -> TransferDownloader {
        TransferDownloader {
            file_id: link_id,
            file_size: file_size as u64,
            chunk_count: chunk_count as u64,
            auth: Auth {
                base_url,
                jwt_token: None,
                refresh_token: None,
                cookie: None,
            },
            decryption_key,
            cipher: cryptfns::cipher::DEFAULT.to_string(),
            public_link: true,
            direct_urls: None,
        }
    }

    /// Set the cipher used to decrypt each chunk.
    /// Accepts `"ascon128a"` (default) or `"chacha20poly1305"`.
    /// Must be called before [`download`].
    #[wasm_bindgen(js_name = "set_cipher")]
    pub fn set_cipher(&mut self, cipher: String) {
        self.cipher = cipher;
    }

    /// Fetch chunks from these presigned bucket URLs, ordered by chunk index,
    /// rather than from this instance.
    ///
    /// The host fetches the manifest because the host is what already reads
    /// `/api/capabilities` and holds the session. Indices the list does not
    /// cover fall back to the API, so a short list degrades rather than
    /// failing. Must be called before `download`.
    ///
    /// Requests built from these URLs carry no cookie, bearer token or
    /// refresh header — see `ChunkTarget`.
    #[wasm_bindgen(js_name = "set_direct_urls")]
    pub fn set_direct_urls(&mut self, urls: Vec<String>) {
        self.direct_urls = Some(urls);
    }

    /// Download and decrypt the file, returning the complete plaintext as a `Uint8Array`.
    ///
    /// Uses a sliding window of concurrent chunk downloads for maximum throughput.
    ///
    /// All configuration fields are cloned before the first `.await`.
    ///
    /// - `on_progress`: JS callback called with a JSON progress string on each chunk.
    /// - `is_cancelled`: JS function `(fileId: string) => boolean`; return `true` to abort.
    pub async fn download(
        &self,
        on_progress: js_sys::Function,
        is_cancelled: js_sys::Function,
    ) -> Result<Vec<u8>, JsValue> {
        // Clone all config fields from &self BEFORE the first await.
        let auth = self.auth.clone();
        let file_id = self.file_id.clone();
        let file_size = self.file_size;
        let chunk_count = self.chunk_count;
        let decryption_key = self.decryption_key.clone();
        let cipher = self.cipher.clone();
        let public_link = self.public_link;
        let direct_urls = self.direct_urls.clone();

        let http = WasmHttpClient::new();
        let reporter = JsProgressReporter::new(on_progress, is_cancelled);

        let mut result = Vec::with_capacity(file_size as usize);
        crate::download::download_file_streaming(
            &http,
            &reporter,
            &auth,
            source_of(&file_id, public_link),
            file_size,
            chunk_count,
            &decryption_key,
            &cipher,
            direct_urls.as_deref(),
            &mut |chunk| result.extend_from_slice(&chunk),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;

        Ok(result)
    }

    /// Download the file, handing each decrypted chunk to `on_chunk` in
    /// file order instead of returning one buffer.
    ///
    /// This is the path for large files: the module's linear memory holds
    /// only the in-flight window (wasm32 caps at 4 GB, and the memory a
    /// buffered download reserves is never returned to the browser), while
    /// the caller parks each chunk in storage the browser manages.
    #[wasm_bindgen(js_name = "downloadStreaming")]
    pub async fn download_streaming(
        &self,
        on_progress: js_sys::Function,
        is_cancelled: js_sys::Function,
        on_chunk: js_sys::Function,
    ) -> Result<(), JsValue> {
        let auth = self.auth.clone();
        let file_id = self.file_id.clone();
        let file_size = self.file_size;
        let chunk_count = self.chunk_count;
        let decryption_key = self.decryption_key.clone();
        let cipher = self.cipher.clone();
        let public_link = self.public_link;
        let direct_urls = self.direct_urls.clone();

        let http = WasmHttpClient::new();
        let reporter = JsProgressReporter::new(on_progress, is_cancelled);

        crate::download::download_file_streaming(
            &http,
            &reporter,
            &auth,
            source_of(&file_id, public_link),
            file_size,
            chunk_count,
            &decryption_key,
            &cipher,
            direct_urls.as_deref(),
            &mut |chunk| {
                let array = js_sys::Uint8Array::from(chunk.as_slice());
                let _ = on_chunk.call1(&JsValue::NULL, &array);
            },
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    /// Download and decrypt a single chunk by index.
    ///
    /// Random access for progressive consumers — video playback feeds
    /// MediaSource one chunk at a time instead of waiting for the file.
    ///
    /// - `on_bytes`: optional JS callback receiving the ciphertext bytes
    ///   received so far for this chunk, from the first network read.
    #[wasm_bindgen(js_name = "downloadChunk")]
    pub async fn download_chunk(
        &self,
        chunk_index: u32,
        on_bytes: Option<js_sys::Function>,
    ) -> Result<Vec<u8>, JsValue> {
        let auth = self.auth.clone();
        let file_id = self.file_id.clone();
        let decryption_key = self.decryption_key.clone();
        let cipher = self.cipher.clone();
        let public_link = self.public_link;

        let http = WasmHttpClient::new();

        // Progressive consumers land here one chunk at a time, so an index the
        // manifest covers goes to the bucket and anything else keeps using the
        // API — same rule the whole-file pipeline follows, including the one
        // about gaps: manifests are index-aligned, so a chunk the server
        // declined to sign arrives as an empty entry that means "through the
        // API", not as a URL to fetch.
        let target = match self
            .direct_urls
            .as_ref()
            .and_then(|urls| urls.get(chunk_index as usize))
            .filter(|url| !url.is_empty())
        {
            Some(url) => ChunkTarget::Direct(url),
            None => ChunkTarget::Api {
                auth: &auth,
                source: source_of(&file_id, public_link),
            },
        };

        let (_, result) = crate::download::fetch_and_decrypt(
            &http,
            target,
            chunk_index as u64,
            &decryption_key,
            &cipher,
            Box::new(move |bytes| {
                if let Some(callback) = &on_bytes {
                    let _ = callback.call1(&JsValue::NULL, &JsValue::from_f64(bytes as f64));
                }
            }),
        )
        .await;

        result.map_err(|e| JsValue::from_str(&format!("{e}")))
    }
}

/// Returns the bitmask value to OR into `set_hash_mask` to disable MD5 computation.
/// The exact ciphertext length of every chunk of a `total_size`-byte file,
/// indexed by chunk.
///
/// These are what `POST /api/storage/{id}/upload-urls` wants declared: the
/// server signs each length into its URL, and the bucket refuses a body of any
/// other size. Computed rather than guessed at the call site, so the sizes
/// cannot drift from what the uploader goes on to produce.
#[wasm_bindgen(js_name = "transferEncryptedChunkSizes")]
pub fn transfer_encrypted_chunk_sizes(cipher: String, total_size: f64) -> Result<Vec<u32>, JsValue> {
    crate::upload::encrypted_chunk_sizes(&cipher, total_size as u64)
        .map(|sizes| sizes.into_iter().map(|size| size as u32).collect())
        .map_err(|e| JsValue::from_str(&format!("{e}")))
}

#[wasm_bindgen(js_name = "transferHashDisableMd5")]
pub fn transfer_hash_disable_md5() -> u32 {
    HASH_DISABLE_MD5
}

/// Returns the bitmask value to OR into `set_hash_mask` to disable SHA-1 computation.
#[wasm_bindgen(js_name = "transferHashDisableSha1")]
pub fn transfer_hash_disable_sha1() -> u32 {
    HASH_DISABLE_SHA1
}

/// Returns the bitmask value to OR into `set_hash_mask` to disable BLAKE2b-512 computation.
#[wasm_bindgen(js_name = "transferHashDisableBlake2b")]
pub fn transfer_hash_disable_blake2b() -> u32 {
    HASH_DISABLE_BLAKE2B
}

/// Returns the bitmask value to OR into `set_hash_mask` to offload SHA-256 to the host.
///
/// When this bit is set, the WASM upload pipeline does **not** compute SHA-256 inline.
/// The host must receive plaintext chunks via `on_plaintext_chunk`, compute the digest
/// externally (e.g. in a dedicated hash Web Worker), and call `PUT .../hashes` after the
/// upload completes.
#[wasm_bindgen(js_name = "transferHashOffloadSha256")]
pub fn transfer_hash_offload_sha256() -> u32 {
    HASH_OFFLOAD_SHA256
}
