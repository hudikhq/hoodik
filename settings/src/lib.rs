pub mod data;
pub mod factory;
pub mod rule;

pub(crate) mod store;

use ::fs::prelude::Filename;
use async_std::sync::{Mutex, MutexGuard};
use async_trait::async_trait;
use std::sync::Arc;

use crate::{data::Data, factory::Factory};

#[derive(Debug, Clone)]
pub struct Settings {
    inner: Arc<Mutex<Data>>,
    mock: bool,
}

impl<'settings> Settings {
    pub fn new(inner: Arc<Mutex<Data>>) -> Self {
        Self { inner, mock: false }
    }

    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Data::default())),
            mock: true,
        }
    }

    /// Get the filename of the settings file.
    pub fn filename() -> Filename {
        Filename::new("settings").with_extension("json")
    }

    /// Get the mutex guard with the inner data structure.
    pub async fn inner<'guard>(&'settings self) -> MutexGuard<'guard, Data>
    where
        'settings: 'guard,
    {
        self.inner.lock().await
    }
}

#[async_trait]
impl Factory for Settings {
    fn memory_only(&self) -> bool {
        self.mock
    }

    async fn replace_inner(&self, inner: Data) {
        *self.inner.lock().await = inner;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Data::default())),
            mock: false,
        }
    }
}

impl From<Data> for Settings {
    fn from(inner: Data) -> Self {
        Settings::from(Arc::new(Mutex::new(inner)))
    }
}

impl From<Mutex<Data>> for Settings {
    fn from(inner: Mutex<Data>) -> Self {
        Settings::from(Arc::new(inner))
    }
}

impl From<Arc<Mutex<Data>>> for Settings {
    fn from(inner: Arc<Mutex<Data>>) -> Self {
        Self::new(inner)
    }
}
