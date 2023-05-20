mod contract;
mod fs;
mod providers;
mod streamer;

/// Maximum chunk a file can have, frontend implementations
/// can choose an arbitrary chunk size up to this size so it
/// could dynamically be adjusted for each file.
pub const MAX_CHUNK_SIZE_BYTES: u64 = 1024 * 1024 * 50;

pub mod prelude {
    pub use super::contract::FsProviderContract;
    pub use super::fs::Fs;
    pub use super::streamer::Streamer;
}
