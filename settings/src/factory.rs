use async_trait::async_trait;
use config::Config;
use error::AppResult;

use crate::data::Data;

#[async_trait]
pub trait Factory: Default + Send + Sync + Sized {
    /// Skip filesystem persistence — useful for tests and ephemeral setups.
    fn memory_only(&self) -> bool {
        false
    }

    async fn replace_inner(&self, inner: Data);

    /// Load settings from disk, write them back round-tripped through the
    /// current schema (so any removed/renamed keys are dropped on first
    /// load), and install the result as the in-memory copy.
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

    async fn refresh(&self, config: &Config) -> AppResult<()> {
        let inner = crate::store::read(config).await?;

        self.replace_inner(inner).await;

        Ok(())
    }

    async fn update(&self, config: &Config, inner: Data) -> AppResult<()> {
        if !self.memory_only() {
            crate::store::write(config, &inner).await?;
        }

        self.replace_inner(inner).await;

        Ok(())
    }
}
