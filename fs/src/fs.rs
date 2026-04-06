use async_trait::async_trait;

use config::Config;
use error::AppResult;

use crate::{
    contract::FsProviderContract,
    filename::IntoFilename,
    providers::fs,
    streamer::Streamer,
};

#[cfg(feature = "s3")]
use crate::providers::s3;

/// Enum dispatch for storage providers. Avoids dynamic dispatch overhead
/// and sidesteps object-safety issues with the generic trait methods.
enum StorageProvider<'a> {
    Local(fs::FsProvider<'a>),
    #[cfg(feature = "s3")]
    S3(Box<s3::S3Provider>),
}

/// Macro to reduce boilerplate for delegating trait methods through the enum.
macro_rules! dispatch {
    ($self:expr, $method:ident ( $($arg:expr),* )) => {
        match $self {
            StorageProvider::Local(p) => p.$method($($arg),*).await,
            #[cfg(feature = "s3")]
            StorageProvider::S3(p) => p.$method($($arg),*).await,
        }
    };
}

#[async_trait]
impl FsProviderContract for StorageProvider<'_> {
    async fn available_space(&self) -> AppResult<u64> {
        dispatch!(self, available_space())
    }

    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>> {
        dispatch!(self, read(filename))
    }

    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()> {
        dispatch!(self, write(filename, data))
    }

    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<bool> {
        dispatch!(self, exists(filename, chunk))
    }

    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i64, data: &[u8]) -> AppResult<()> {
        dispatch!(self, push(filename, chunk, data))
    }

    async fn pull<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<Vec<u8>> {
        dispatch!(self, pull(filename, chunk))
    }

    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        dispatch!(self, purge(filename))
    }

    async fn get_uploaded_chunks<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<i64>> {
        dispatch!(self, get_uploaded_chunks(filename))
    }

    async fn stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i64>,
    ) -> AppResult<Streamer> {
        dispatch!(self, stream(filename, chunk))
    }

    async fn stream_tar<T: IntoFilename>(&self, filename: &T) -> AppResult<Streamer> {
        dispatch!(self, stream_tar(filename))
    }

    async fn tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64> {
        dispatch!(self, tar_content_length(filename))
    }
}

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

    /// Default storage provider based on application configuration.
    fn provider(&self) -> StorageProvider<'_> {
        match self.config.app.storage_provider.as_str() {
            #[cfg(feature = "s3")]
            "s3" => {
                let s3_config = self
                    .config
                    .s3
                    .as_ref()
                    .expect("S3 config is required when STORAGE_PROVIDER=s3");
                StorageProvider::S3(Box::new(s3::S3Provider::new(s3_config)))
            }
            #[cfg(not(feature = "s3"))]
            "s3" => {
                panic!(
                    "STORAGE_PROVIDER=s3 is set but the 's3' feature is not enabled. \
                     Rebuild with: cargo build --features fs/s3"
                );
            }
            _ => StorageProvider::Local(fs::FsProvider::new(&self.config.app.data_dir)),
        }
    }
}

#[async_trait]
impl FsProviderContract for Fs<'_> {
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

    async fn stream_tar<T: IntoFilename>(&self, filename: &T) -> AppResult<Streamer> {
        self.provider().stream_tar(filename).await
    }

    async fn tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64> {
        self.provider().tar_content_length(filename).await
    }
}
