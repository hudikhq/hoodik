use std::fs::File;

use error::AppResult;

pub trait StorageProvider {
    /// Check if the chunk already exists in the storage provider
    fn part_exists(&self, filename: &str, chunk: i32) -> AppResult<bool>;

    /// Get a representation of a file from the storage provider.
    fn get(&self, filename: &str) -> AppResult<File>;

    /// Create a new file in the storage provider.
    fn create(&self, filename: &str) -> AppResult<File>;

    /// Get or create a file in the storage provider.
    fn get_or_create(&self, filename: &str) -> AppResult<File>;

    /// Push data to a file in the storage provider.
    fn push(&self, filename: &str, data: &[u8]) -> AppResult<()>;

    /// Push specific data chunk into a part file for uploading file
    fn push_part(&self, filename: &str, chunk: i32, data: &[u8]) -> AppResult<()>;

    /// Pull data chunk from a file in the storage provider.
    /// Chunk is calculated by dividing the original file size by the
    /// CHUNK_SIZE_BYTES constant.
    fn pull(&self, filename: &str, chunk: u64) -> AppResult<Vec<u8>>;

    /// Remove a file in storage provider
    fn remove(&self, filename: &str) -> AppResult<()>;

    /// Concatenate all the part files into a single file
    fn concat_files(&self, filename: &str, chunks: u64) -> AppResult<()>;
}
