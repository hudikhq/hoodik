use crate::{filename::IntoFilename, streamer::Streamer};
use error::AppResult;

use async_trait::async_trait;
use tokio::fs::File;

#[async_trait]
pub trait FsProviderContract {
    /// Get the available space on the storage provider
    async fn available_space(&self) -> AppResult<u64>;

    /// Direct read of the file data
    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>>;

    /// Direct write of the file data
    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()>;

    /// Check if the chunk already exists in the storage provider
    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i32) -> AppResult<bool>;

    /// Get a file representation from the storage provider
    async fn get<T: IntoFilename>(&self, filename: &T, chunk: i32) -> AppResult<File>;

    /// Get a file representation from the storage provider
    async fn all<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<File>>;

    /// Push specific data chunk into a part file
    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i32, data: &[u8]) -> AppResult<()>;

    /// Pull data chunk of a file from the storage provider.
    async fn pull<T: IntoFilename>(&self, filename: &T, chunk: i32) -> AppResult<Vec<u8>>;

    /// Purge all the parts for a file from the storage provider.
    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()>;

    /// Get a vector of chunk indexes that were already uploaded so we can resume
    /// the upload process on the frontend without doing the double work.
    async fn get_uploaded_chunks<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<i32>>;

    /// Return stream of either one file chunk, or all chunks if no file chunk is specified.
    async fn stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i32>,
    ) -> AppResult<Streamer>;
}
