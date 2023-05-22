use chrono::NaiveDateTime;
use entity::Uuid;
use error::AppResult;
use std::fmt::{Display, Formatter, Result};

/// Struct representing the filename which will be used
/// so that different types can implement a TryInto trait
/// when trying to use the fs.
#[derive(Clone, Debug)]
pub struct Filename {
    created_at: NaiveDateTime,
    file_id: Uuid,
    extension: Option<String>,
    chunk: Option<i32>,
}

impl Display for Filename {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let timestamp = self.created_at.timestamp();
        let file_id = self.file_id.to_string();

        let part = self
            .chunk
            .map(|c| format!(".part.{}", c))
            .unwrap_or_default();

        let extension = self.chunk.map(|c| format!(".{}", c)).unwrap_or_default();

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

    pub fn with_chunk(mut self, chunk: i32) -> Self {
        self.chunk = Some(chunk);
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
