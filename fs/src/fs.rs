use async_trait::async_trait;
use tokio::fs::File;

use config::Config;
use error::AppResult;

use crate::{contract::FsProviderContract, providers::fs, streamer::Streamer};

pub struct Fs<'ctx> {
    config: &'ctx Config,
}

impl<'ctx> Fs<'ctx> {
    pub fn new(config: &'ctx Config) -> Self {
        Self { config }
    }

    fn provider<'provider>(&self) -> impl FsProviderContract + 'provider
    where
        'ctx: 'provider,
    {
        fs::FsProvider::<'provider>::new(&self.config.data_dir)
    }
}

#[async_trait]
impl<'ctx> FsProviderContract for Fs<'ctx> {
    async fn exists(&self, filename: &str, chunk: i32) -> AppResult<bool> {
        self.provider().exists(filename, chunk).await
    }

    async fn get(&self, filename: &str, chunk: i32) -> AppResult<File> {
        self.provider().get(filename, chunk).await
    }

    async fn all(&self, filename: &str) -> AppResult<Vec<File>> {
        self.provider().all(filename).await
    }

    async fn push(&self, filename: &str, chunk: i32, data: &[u8]) -> AppResult<()> {
        self.provider().push(filename, chunk, data).await
    }

    async fn pull(&self, filename: &str, chunk: i32) -> AppResult<Vec<u8>> {
        self.provider().pull(filename, chunk).await
    }

    async fn purge(&self, filename: &str) -> AppResult<()> {
        self.provider().purge(filename).await
    }

    async fn get_uploaded_chunks(&self, filename: &str) -> AppResult<Vec<i32>> {
        self.provider().get_uploaded_chunks(filename).await
    }

    async fn stream(&self, filename: &str, chunk: Option<i32>) -> Streamer {
        self.provider().stream(filename, chunk).await
    }
}
