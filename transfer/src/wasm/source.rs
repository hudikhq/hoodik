use crate::error::{Error, Result};
use crate::platform::DataSource;
use wasm_bindgen_futures::JsFuture;

/// Wraps a browser `File` object as a `DataSource`.
pub struct FileSource {
    file: web_sys::File,
    size: u64,
}

impl FileSource {
    pub fn new(file: web_sys::File) -> Self {
        let size = file.size() as u64;
        Self { file, size }
    }
}

impl DataSource for FileSource {
    fn read_chunk(
        &self,
        offset: u64,
        length: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>> {
        let start = offset as f64;
        let end = (offset + length) as f64;

        Box::pin(async move {
            let blob = self
                .file
                .slice_with_f64_and_f64(start, end)
                .map_err(|e| Error::Io(format!("File.slice failed: {e:?}")))?;

            let array_buffer = JsFuture::from(
                blob.array_buffer(),
            )
            .await
            .map_err(|e| Error::Io(format!("arrayBuffer failed: {e:?}")))?;

            let uint8 = js_sys::Uint8Array::new(&array_buffer);
            Ok(uint8.to_vec())
        })
    }

    fn total_size(&self) -> u64 {
        self.size
    }
}
