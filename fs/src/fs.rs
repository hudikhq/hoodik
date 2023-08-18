use async_trait::async_trait;
use tokio::fs::File;

use config::Config;
use error::AppResult;

use crate::{
    contract::FsProviderContract, filename::IntoFilename, providers::fs, streamer::Streamer,
};

pub struct Fs<'ctx> {
    config: &'ctx Config,
}

impl<'ctx> Fs<'ctx> {
    pub fn new(config: &'ctx Config) -> Self {
        Self { config }
    }

    /// Local file system provider for rw operations on the
    /// local machine filesystem.
    pub fn local<'provider>(&self) -> impl FsProviderContract + 'provider
    where
        'ctx: 'provider,
    {
        self.local_in(&self.config.app.data_dir)
    }

    /// Local file system provider with provided data_dir
    pub fn local_in<'provider>(
        &self,
        data_dir: &'provider str,
    ) -> impl FsProviderContract + 'provider
    where
        'ctx: 'provider,
    {
        fs::FsProvider::<'provider>::new(data_dir)
    }

    /// Default storage provider for rw operations on either local, or any
    /// other provider that the application configuration specifies.
    fn provider<'provider>(&self) -> impl FsProviderContract + 'provider
    where
        'ctx: 'provider,
    {
        // TODO: Use the config to decide which provider we will be using
        // for file storage. Once S3 is implemented...
        self.local_in(&self.config.app.data_dir)
    }
}

#[async_trait]
impl<'ctx> FsProviderContract for Fs<'ctx> {
    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>> {
        self.provider().read(filename).await
    }

    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()> {
        self.provider().write(filename, data).await
    }

    async fn available_space(&self) -> AppResult<u64> {
        self.provider().available_space().await
    }

    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<bool> {
        self.provider().exists(filename, chunk).await
    }

    async fn get<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<File> {
        self.provider().get(filename, chunk).await
    }

    async fn all<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<File>> {
        self.provider().all(filename).await
    }

    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i64, data: &[u8]) -> AppResult<()> {
        self.provider().push(filename, chunk, data).await
    }

    async fn pull<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<Vec<u8>> {
        self.provider().pull(filename, chunk).await
    }

    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        self.provider().purge(filename).await
    }

    async fn get_uploaded_chunks<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<i64>> {
        self.provider().get_uploaded_chunks(filename).await
    }

    async fn stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i64>,
    ) -> AppResult<Streamer> {
        self.provider().stream(filename, chunk).await
    }
}
