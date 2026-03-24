/// Size of a single chunk in bytes (4 MiB).
/// Must match the backend's MAX_CHUNK_SIZE_BYTES.
pub const CHUNK_SIZE_BYTES: u64 = 4 * 1024 * 1024;

/// Maximum number of retry attempts on checksum mismatch.
pub const MAX_UPLOAD_RETRIES: u32 = 3;

/// Encrypted chunk uploads in flight at once. Raised from 6 so encryption can keep feeding uploads
/// instead of stalling on backpressure.
pub const CONCURRENT_UPLOADS_IN_FLIGHT: usize = 8;

/// Maximum number of encrypted chunks buffered in memory (uploading + waiting to upload).
pub const ENCRYPTED_CHUNKS_BUFFER_LIMIT: usize = 24;

/// Log upload pipeline timing to stderr (native) / browser console (WASM). Off during `cargo test`.
pub const UPLOAD_PIPELINE_TRACE: bool = false;

/// Focused tracing around hashing/offload correctness (not the whole upload pipeline).
pub const HASH_TRACE: bool = false;

/// Number of chunks downloaded in parallel per file.
pub const DOWNLOAD_POOL_LIMIT: usize = 16;

/// Disable MD5 when set (web upload currently ORs 1|2|4 for SHA-256–only).
pub const HASH_DISABLE_MD5: u32 = 1;
/// Disable SHA-1 when set.
pub const HASH_DISABLE_SHA1: u32 = 2;
/// Disable BLAKE2b-512 when set.
pub const HASH_DISABLE_BLAKE2B: u32 = 4;
/// When set, do not compute file SHA-256 (or optional hashes) in the upload WASM thread; the host
/// must hash plaintext via [`crate::upload::PlaintextChunkHook`] (e.g. a Web Worker).
pub const HASH_OFFLOAD_SHA256: u32 = 8;

/// Which optional content hashes to compute during upload. SHA-256 is always computed unless
/// [`Self::inline_sha256`] is false (offload).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UploadHashOptions {
    pub md5: bool,
    pub sha1: bool,
    pub blake2b: bool,
    /// When false, file SHA-256 is not computed in-process; a [`crate::upload::PlaintextChunkHook`]
    /// must receive each chunk in order.
    pub inline_sha256: bool,
}

impl Default for UploadHashOptions {
    fn default() -> Self {
        Self {
            md5: true,
            sha1: true,
            blake2b: true,
            inline_sha256: true,
        }
    }
}

impl UploadHashOptions {
    /// Build from a bitmask OR of [`HASH_DISABLE_MD5`], [`HASH_DISABLE_SHA1`], [`HASH_DISABLE_BLAKE2B`],
    /// and optionally [`HASH_OFFLOAD_SHA256`].
    /// `0` means all optional hashes enabled and SHA-256 computed inline (production default).
    pub fn from_disable_mask(mask: u32) -> Self {
        Self {
            md5: (mask & HASH_DISABLE_MD5) == 0,
            sha1: (mask & HASH_DISABLE_SHA1) == 0,
            blake2b: (mask & HASH_DISABLE_BLAKE2B) == 0,
            inline_sha256: (mask & HASH_OFFLOAD_SHA256) == 0,
        }
    }
}
