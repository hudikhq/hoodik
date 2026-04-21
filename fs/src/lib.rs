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
