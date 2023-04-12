use std::fs::File;

use config::Config;
use error::AppResult;

use crate::{contract::StorageProvider, providers::fs};

pub struct Storage<'ctx> {
    config: &'ctx Config,
}

impl<'ctx> Storage<'ctx> {
    pub fn new(config: &'ctx Config) -> Self {
        Self { config }
    }

    fn provider<'provider>(&self) -> impl StorageProvider + 'provider
    where
        'ctx: 'provider,
    {
        fs::FsProvider::<'provider>::new(&self.config.data_dir)
    }
}

impl<'ctx> StorageProvider for Storage<'ctx> {
    fn part_exists(&self, filename: &str, chunk: i32) -> AppResult<bool> {
        self.provider().part_exists(filename, chunk)
    }

    fn get(&self, filename: &str) -> AppResult<File> {
        self.provider().get(filename)
    }

    fn create(&self, filename: &str) -> AppResult<File> {
        self.provider().create(filename)
    }

    fn get_or_create(&self, filename: &str) -> AppResult<File> {
        self.provider().get_or_create(filename)
    }

    fn push(&self, filename: &str, data: &[u8]) -> AppResult<()> {
        self.provider().push(filename, data)
    }

    fn push_part(&self, filename: &str, chunk: i32, data: &[u8]) -> AppResult<()> {
        self.provider().push_part(filename, chunk, data)
    }

    fn pull(&self, filename: &str, chunk: u64) -> AppResult<Vec<u8>> {
        self.provider().pull(filename, chunk)
    }

    fn remove(&self, filename: &str) -> AppResult<()> {
        self.provider().remove(filename)
    }

    fn concat_files(&self, filename: &str, chunks: u64) -> AppResult<()> {
        self.provider().concat_files(filename, chunks)
    }
}
