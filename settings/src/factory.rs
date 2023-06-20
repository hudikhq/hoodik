use async_trait::async_trait;
use config::Config;
use error::AppResult;

use crate::data::Data;

#[async_trait]
pub trait Factory: Default + Send + Sync + Sized {
    /// Method that tells the storage actions if they should save the data
    fn memory_only(&self) -> bool {
        false
    }

    /// Implementation of the inner replacement with a new inner data.
    async fn replace_inner(&self, inner: Data);

    /// Attempt to load the configuration from the filesystem.
    ///
    /// After the configuration is parsed it will be wrapped in a `Settings` struct
    /// and it will be stored back on the filesystem so it is clear of any wrong, or
    /// outdated information. This will also be useful in the future to update any
    /// new settings that will be added.
    async fn create(self, config: &Config) -> AppResult<Self> {
        if self.memory_only() {
            return Ok(self);
        }

        let inner = crate::store::read(config).await.unwrap_or_else(|e| {
            log::warn!("Failed to read settings from filesystem: {}", e);

            Data::default()
        });

        crate::store::write(config, &inner).await?;

        self.replace_inner(inner).await;

        Ok(self)
    }

    /// Refresh the inner data with the data from the filesystem.
    async fn refresh(&self, config: &Config) -> AppResult<()> {
        let inner = crate::store::read(config).await?;

        self.replace_inner(inner).await;

        Ok(())
    }

    /// Replace the inner data with the new data, and persist it to the disk
    async fn update(&self, config: &Config, inner: Data) -> AppResult<()> {
        if !self.memory_only() {
            crate::store::write(config, &inner).await?;
        }

        self.replace_inner(inner).await;

        Ok(())
    }
}
