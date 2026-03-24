use crate::platform::ProgressReporter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Callback-based progress reporter for native (Flutter/desktop) use.
///
/// The `on_progress` closure receives a JSON string with each update.
/// Cancellation is signalled by setting the `cancelled` flag from the host.
pub struct NativeProgressReporter<F: Fn(&str)> {
    on_progress: F,
    cancelled: Arc<AtomicBool>,
}

impl<F: Fn(&str)> NativeProgressReporter<F> {
    pub fn new(on_progress: F, cancelled: Arc<AtomicBool>) -> Self {
        Self {
            on_progress,
            cancelled,
        }
    }
}

impl<F: Fn(&str)> ProgressReporter for NativeProgressReporter<F> {
    fn on_chunk_uploaded(&self, file_id: &str, chunk: u64, total_chunks: u64, is_done: bool) {
        let json = serde_json::json!({
            "type": "upload",
            "file_id": file_id,
            "chunk": chunk,
            "total_chunks": total_chunks,
            "is_done": is_done,
        });
        (self.on_progress)(&json.to_string());
    }

    fn on_chunk_downloaded(&self, file_id: &str, bytes: u64, total_bytes: u64) {
        let json = serde_json::json!({
            "type": "download",
            "file_id": file_id,
            "bytes_downloaded": bytes,
            "total_bytes": total_bytes,
        });
        (self.on_progress)(&json.to_string());
    }

    fn on_error(&self, file_id: &str, error: &str) {
        let json = serde_json::json!({
            "type": "error",
            "file_id": file_id,
            "error": error,
        });
        (self.on_progress)(&json.to_string());
    }

    fn on_complete(&self, file_id: &str) {
        let json = serde_json::json!({
            "type": "complete",
            "file_id": file_id,
        });
        (self.on_progress)(&json.to_string());
    }

    fn is_cancelled(&self, _file_id: &str) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}
