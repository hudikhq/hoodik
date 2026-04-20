pub mod checksum;
pub mod config;
pub(crate) mod upload_trace;
pub mod download;
pub mod error;
pub mod platform;
pub mod tar;
pub mod types;
pub mod upload;
pub mod upload_tar;

// Re-export the primary public API for library consumers.
pub use download::Downloader;
pub use upload::{PlaintextChunkHook, Uploader};

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub use wasm::{TransferBlake2b512, TransferMd5, TransferSha1, TransferSha256};

#[cfg(feature = "wasm")]
pub use wasm::api::{TransferDownloader, TransferUploader};

#[cfg(any(feature = "native", feature = "mobile"))]
pub mod native;

#[cfg(test)]
mod tests;
