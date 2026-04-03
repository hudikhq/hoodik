use crate::error::Result;
use crate::types::{Auth, ChunkResponse, FileHashes};

/// Platform-agnostic HTTP client for upload/download chunk requests.
pub trait HttpClient {
    /// Upload a single encrypted chunk. Returns server metadata about stored chunks.
    fn upload_chunk(
        &self,
        auth: &Auth,
        file_id: &str,
        chunk_index: u64,
        checksum: &str,
        data: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ChunkResponse>> + '_>>;

    /// Download a single encrypted chunk as raw bytes.
    fn download_chunk(
        &self,
        auth: &Auth,
        file_id: &str,
        chunk_index: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>>;

    /// Download all chunks as a tar archive containing individual encrypted chunk files.
    /// Each entry is named `{chunk_index:06}.enc`. Returns the raw tar bytes.
    fn download_all_chunks(
        &self,
        auth: &Auth,
        file_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>>;

    /// Send computed file hashes to the server after upload completes.
    fn update_hashes(
        &self,
        auth: &Auth,
        file_id: &str,
        hashes: &FileHashes,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + '_>>;
}

/// Platform-agnostic source of file data (browser File, native fs, etc.).
pub trait DataSource {
    /// Read `length` bytes starting at `offset`. Returns the bytes read.
    fn read_chunk(
        &self,
        offset: u64,
        length: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>>;

    /// Total size of the source in bytes.
    fn total_size(&self) -> u64;
}

/// Callback interface for progress updates and cooperative cancellation.
pub trait ProgressReporter {
    fn on_chunk_uploaded(&self, file_id: &str, chunk: u64, total_chunks: u64, is_done: bool);
    fn on_chunk_downloaded(&self, file_id: &str, bytes: u64, total_bytes: u64);
    fn on_error(&self, file_id: &str, error: &str);
    fn on_complete(&self, file_id: &str);
    fn is_cancelled(&self, file_id: &str) -> bool;
}
