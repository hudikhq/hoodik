use crate::{filename::IntoFilename, streamer::Streamer};
use error::AppResult;

use async_trait::async_trait;

#[async_trait]
pub trait FsProviderContract {
    /// Get the available space on the storage provider
    async fn available_space(&self) -> AppResult<u64>;

    /// Direct read of the file data
    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>>;

    /// Direct write of the file data
    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()>;

    /// Check if the chunk already exists in the storage provider
    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<bool>;

    /// Push specific data chunk into a part file
    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i64, data: &[u8]) -> AppResult<()>;

    /// Pull data chunk of a file from the storage provider.
    async fn pull<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<Vec<u8>>;

    /// Purge all the parts for a file from the storage provider.
    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()>;

    /// Get a vector of chunk indexes that were already uploaded so we can resume
    /// the upload process on the frontend without doing the double work.
    async fn get_uploaded_chunks<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<i64>>;

    /// Return stream of either one file chunk, or all chunks if no file chunk is specified.
    async fn stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i64>,
    ) -> AppResult<Streamer>;

    /// Stream all chunks as an uncompressed tar archive. Each chunk becomes a
    /// separate entry named `{chunk_index:06}.enc`, preserving chunk boundaries.
    async fn stream_tar<T: IntoFilename>(&self, filename: &T) -> AppResult<Streamer>;

    /// Calculate the total size of the tar archive without streaming it.
    async fn tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64>;

    // Read methods fall back to the legacy flat layout
    // (`{timestamp}-{uuid}.part.{n}`) when version == 1 and the new
    // versioned directory is empty, so pre-migration files keep serving
    // downloads; their first edit upgrades them by landing into `v2/`
    // and recording a `file_versions(version=1)` row for the legacy set
    // in the storage repository layer.
    //
    // Writes go straight to the new layout
    // (`{uuid}/v{version}/{chunk:06}.chunk`) — legacy is read-only.

    /// Write a chunk into a specific version's directory.
    async fn push_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
        data: &[u8],
    ) -> AppResult<()>;

    /// Read a chunk from a specific version. Falls back to legacy path
    /// for `version == 1` if the versioned dir has no chunks.
    async fn pull_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
    ) -> AppResult<Vec<u8>>;

    /// Check whether a specific chunk exists in the given version.
    /// Same legacy fallback as `pull_v`.
    async fn exists_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
    ) -> AppResult<bool>;

    /// List uploaded chunk indices for a specific version. Returns an
    /// empty Vec when the version directory does not exist. Legacy
    /// fallback applies for `version == 1`.
    async fn get_uploaded_chunks_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<Vec<i64>>;

    /// Stream a single chunk or all chunks of a specific version.
    async fn stream_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: Option<i64>,
    ) -> AppResult<Streamer>;

    /// Stream all chunks of a specific version as an uncompressed tar.
    async fn stream_tar_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<Streamer>;

    /// Total tar size for a specific version, without streaming.
    async fn tar_content_length_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<u64>;

    /// Delete a single version's directory and its chunks. No-op if the
    /// directory does not exist (recovery case after a half-finished
    /// abandon).
    async fn purge_version<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<()>;

    /// Copy all chunks from `src/v{src_version}/` to
    /// `dst/v{dst_version}/`. Source and destination filenames are
    /// independent — pass the same one for restore-in-place, different
    /// ones for fork-as-new-note.
    async fn copy_version<S: IntoFilename, D: IntoFilename>(
        &self,
        src: &S,
        src_version: i32,
        dst: &D,
        dst_version: i32,
    ) -> AppResult<()>;

    /// Delete the file's entire on-disk footprint — every version
    /// directory plus any legacy chunks. Used on full file deletion.
    async fn purge_all<T: IntoFilename>(&self, filename: &T) -> AppResult<()>;
}
