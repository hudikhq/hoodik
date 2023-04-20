use async_trait::async_trait;
use error::AppResult;
use tokio::fs::File;

use crate::streamer::Streamer;

#[async_trait]
pub trait StorageProvider {
    /// Check if the chunk already exists in the storage provider
    async fn exists(&self, filename: &str, chunk: i32) -> AppResult<bool>;

    /// Get a file representation from the storage provider
    async fn get(&self, filename: &str, chunk: i32) -> AppResult<File>;

    /// Get a file representation from the storage provider
    async fn all(&self, filename: &str) -> AppResult<Vec<File>>;

    /// Push specific data chunk into a part file
    async fn push(&self, filename: &str, chunk: i32, data: &[u8]) -> AppResult<()>;

    /// Pull data chunk of a file from the storage provider.
    async fn pull(&self, filename: &str, chunk: i32) -> AppResult<Vec<u8>>;

    /// Purge all the parts for a file from the storage provider.
    async fn purge(&self, filename: &str) -> AppResult<()>;

    /// Get a vector of chunk indexes that were already uploaded so we can resume
    /// the upload process on the frontend without doing the double work.
    async fn get_uploaded_chunks(&self, filename: &str) -> AppResult<Vec<i32>>;

    /// Return stream of either one file chunk, or all chunks if no file chunk is specified.
    async fn stream(&self, filename: &str, chunk: Option<i32>) -> Streamer;
}
