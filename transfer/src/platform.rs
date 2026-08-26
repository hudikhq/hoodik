use crate::error::Result;
use crate::types::{Auth, ChunkResponse, ChunkTarget, FileHashes};

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
    ///
    /// The [`ChunkTarget`] decides both where the request goes and whether it
    /// may carry credentials — see that type. Implementations must not reach
    /// for auth outside what the target hands them.
    ///
    /// `on_bytes` is invoked with the number of bytes received *so far for
    /// this chunk* as the response body streams in — starting from the first
    /// read, not from chunk completion. Implementations that cannot stream
    /// call it once with the full length; a restarted request starts the
    /// count over, which is what lets the caller keep an accurate cumulative
    /// total across retries.
    fn download_chunk<'a>(
        &'a self,
        target: ChunkTarget<'_>,
        chunk_index: u64,
        on_bytes: Box<dyn Fn(u64) + 'a>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + 'a>>;

    /// Download all chunks as a tar archive containing individual encrypted chunk files.
    /// Each entry is named `{chunk_index:06}.enc`. Returns the raw tar bytes.
    ///
    /// `on_bytes` follows the same contract as [`Self::download_chunk`],
    /// counting the tar body itself.
    fn download_all_chunks<'a>(
        &'a self,
        auth: &Auth,
        file_id: &str,
        on_bytes: Box<dyn Fn(u64) + 'a>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + 'a>>;

    /// Upload multiple chunks in a single request as a tar archive.
    ///
    /// The server endpoint (`POST /api/storage/{file_id}?format=tar`) unpacks
    /// each entry into the file's chunk storage. Returns the same
    /// `ChunkResponse` shape as [`Self::upload_chunk`] — callers can treat the
    /// two paths as interchangeable at the response level.
    fn upload_chunks_tar(
        &self,
        auth: &Auth,
        file_id: &str,
        tar_body: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ChunkResponse>> + '_>>;

    /// Write one encrypted chunk straight into the storage bucket.
    ///
    /// The presigned URL is the whole credential: it is signed over the
    /// method, the object key and the exact content length, so nothing about
    /// the session may be attached. Implementations must send no
    /// `Authorization` header and no cookies, for the reasons spelled out on
    /// [`crate::types::ChunkTarget::Direct`].
    fn put_chunk_direct(
        &self,
        url: &str,
        data: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + '_>>;

    /// Commit a file whose chunks were written directly.
    ///
    /// The relaying routes finalize themselves once their own writes complete
    /// the count. Nothing tells the server that a bucket write landed, so the
    /// client says so — and the server lists the bucket to confirm it before
    /// the version pointer moves.
    fn finalize_upload(
        &self,
        auth: &Auth,
        file_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + '_>>;

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
