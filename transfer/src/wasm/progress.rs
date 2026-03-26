use crate::platform::ProgressReporter;
use wasm_bindgen::prelude::*;

/// Wraps two JS callback functions to implement `ProgressReporter`.
///
/// `on_progress` is called with a JSON string of the progress update.
/// `is_cancelled_fn` is called with the file_id and should return a boolean.
pub struct JsProgressReporter {
    on_progress: js_sys::Function,
    is_cancelled_fn: js_sys::Function,
}

impl JsProgressReporter {
    pub fn new(on_progress: js_sys::Function, is_cancelled_fn: js_sys::Function) -> Self {
        Self {
            on_progress,
            is_cancelled_fn,
        }
    }

    fn call_progress(&self, json: &str) {
        let _ = self
            .on_progress
            .call1(&JsValue::NULL, &JsValue::from_str(json));
    }
}

impl ProgressReporter for JsProgressReporter {
    fn on_chunk_uploaded(&self, file_id: &str, chunk: u64, total_chunks: u64, is_done: bool) {
        let json = serde_json::json!({
            "type": "upload",
            "file_id": file_id,
            "chunk": chunk,
            "total_chunks": total_chunks,
            "is_done": is_done,
        });
        self.call_progress(&json.to_string());
    }

    fn on_chunk_downloaded(&self, file_id: &str, bytes: u64, total_bytes: u64) {
        let json = serde_json::json!({
            "type": "download",
            "file_id": file_id,
            "bytes_downloaded": bytes,
            "total_bytes": total_bytes,
        });
        self.call_progress(&json.to_string());
    }

    fn on_error(&self, file_id: &str, error: &str) {
        let json = serde_json::json!({
            "type": "error",
            "file_id": file_id,
            "error": error,
        });
        self.call_progress(&json.to_string());
    }

    fn on_complete(&self, file_id: &str) {
        let json = serde_json::json!({
            "type": "complete",
            "file_id": file_id,
        });
        self.call_progress(&json.to_string());
    }

    fn is_cancelled(&self, file_id: &str) -> bool {
        self.is_cancelled_fn
            .call1(&JsValue::NULL, &JsValue::from_str(file_id))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}
