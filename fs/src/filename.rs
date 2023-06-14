use chrono::NaiveDateTime;
use error::AppResult;
use std::fmt::{Display, Formatter, Result};
use uuid::Uuid;

/// Struct representing the filename which will be used
/// so that different types can implement a TryInto trait
/// when trying to use the fs.
#[derive(Clone, Debug)]
pub struct Filename {
    created_at: NaiveDateTime,
    file_id: Uuid,
    extension: Option<String>,
    chunk: Option<String>,
}

impl Display for Filename {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let timestamp = self.created_at.timestamp();
        let file_id = self.file_id.to_string();

        let part = self
            .chunk
            .as_ref()
            .map(|c| format!(".part.{}", c))
            .unwrap_or_default();

        let extension = self
            .extension
            .as_ref()
            .map(|c| format!(".{}", c))
            .unwrap_or_default();

        write!(f, "{timestamp}-{file_id}{part}{extension}")
    }
}

impl Filename {
    pub fn new(created_at: NaiveDateTime, file_id: Uuid) -> Self {
        Self {
            created_at,
            file_id,
            extension: None,
            chunk: None,
        }
    }

    pub fn with_extension<T: ToString>(mut self, extension: T) -> Self {
        self.extension = Some(extension.to_string());
        self
    }

    pub fn with_chunk<T: ToString>(mut self, chunk: T) -> Self {
        self.chunk = Some(chunk.to_string());
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
