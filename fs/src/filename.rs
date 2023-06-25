use error::AppResult;
use std::fmt::{Display, Formatter, Result};

/// Struct representing the filename which will be used
/// so that different types can implement a TryInto trait
/// when trying to use the fs.
#[derive(Clone, Debug)]
pub struct Filename {
    timestamp: Option<String>,
    inner_name: String,
    extension: Option<String>,
    chunk: Option<String>,
}

impl Display for Filename {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}{}{}{}",
            self.timestamp.as_deref().unwrap_or(""),
            self.inner_name,
            self.chunk.as_deref().unwrap_or(""),
            self.extension.as_deref().unwrap_or("")
        )
    }
}

impl Filename {
    pub fn new<T: ToString>(name: T) -> Self {
        Self {
            timestamp: None,
            inner_name: name.to_string(),
            extension: None,
            chunk: None,
        }
    }

    pub fn with_timestamp<T: ToString>(mut self, timestamp: T) -> Self {
        self.timestamp = Some(format!("{}-", timestamp.to_string()));

        self
    }

    pub fn with_extension<T: ToString>(mut self, extension: T) -> Self {
        self.extension = Some(format!(".{}", extension.to_string()));

        self
    }

    pub fn with_chunk<T: ToString>(mut self, chunk: T) -> Self {
        self.chunk = Some(format!(".part.{}", chunk.to_string()));

        self
    }
}

/// Trait to implement on file representations
pub trait IntoFilename
where
    Self: Send + Sync,
{
    /// Get the filename from the file representation
    fn filename(&self) -> AppResult<Filename>;
}

impl IntoFilename for Filename {
    fn filename(&self) -> AppResult<Filename> {
        Ok(self.clone())
    }
}
