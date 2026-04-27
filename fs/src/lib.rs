mod contract;
mod filename;
mod fs;
mod providers;
mod streamer;
pub mod tar;

pub use filename::IntoFilename;

/// Maximum chunk a file can have, frontend implementations
/// can choose an arbitrary chunk size up to this size so it
/// could dynamically be adjusted for each file.
pub const MAX_CHUNK_SIZE_BYTES: u64 = 1024 * 1024 * 4;

/// Maximum encrypted body the upload routes accept for one chunk: a full
/// [`MAX_CHUNK_SIZE_BYTES`] plaintext plus 1 % slack for the AEAD tag and
/// nonce/header overhead the client appends. Every cipher we ship adds 16
/// bytes of tag (AEGIS-128L, ChaCha20-Poly1305, Ascon-128a), so 1 % is far
/// more than the worst case but keeps the per-chunk and tar wire ceilings
/// in lockstep with one shared constant.
pub const MAX_CHUNK_PAYLOAD_BYTES: u64 =
    MAX_CHUNK_SIZE_BYTES + MAX_CHUNK_SIZE_BYTES / 100;

// S3 CopyObject's single-operation limit is 5 GiB. Any chunk larger than that
// would force the S3 `copy_version` path into a multipart-copy code path we
// don't have — static-check it now so a future bump of MAX_CHUNK_SIZE_BYTES
// fails the build here instead of silently breaking S3 restore/fork.
const _: () = assert!(MAX_CHUNK_SIZE_BYTES < 5 * 1024 * 1024 * 1024);

pub mod prelude {
    pub use super::contract::FsProviderContract;
    pub use super::filename::{Filename, IntoFilename};
    pub use super::fs::Fs;
    pub use super::streamer::Streamer;

    #[cfg(feature = "s3")]
    pub use super::providers::s3::S3Provider;
}
