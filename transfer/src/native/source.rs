use crate::error::{Error, Result};
use crate::platform::DataSource;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Reads file data from the native file system via `tokio::fs`.
pub struct FileSource {
    path: PathBuf,
    size: u64,
}

impl FileSource {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|e| Error::Io(format!("Failed to read file metadata: {e}")))?;

        Ok(Self {
            path,
            size: metadata.len(),
        })
    }
}

impl DataSource for FileSource {
    fn read_chunk(
        &self,
        offset: u64,
        length: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>> {
        Box::pin(async move {
            let mut file = tokio::fs::File::open(&self.path)
                .await
                .map_err(|e| Error::Io(format!("Failed to open file: {e}")))?;

            file.seek(std::io::SeekFrom::Start(offset))
                .await
                .map_err(|e| Error::Io(format!("Failed to seek: {e}")))?;

            let mut buf = vec![0u8; length as usize];
            let bytes_read = file
                .read_exact(&mut buf)
                .await
                .map_err(|e| Error::Io(format!("Failed to read chunk: {e}")))?;

            buf.truncate(bytes_read);
            Ok(buf)
        })
    }

    fn total_size(&self) -> u64 {
        self.size
    }
}
