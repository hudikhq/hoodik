//! Optional timing logs for upload pipelining (read/hash vs encrypt vs HTTP vs backpressure).

use crate::config::{HASH_TRACE, UPLOAD_PIPELINE_TRACE};

pub(crate) fn now_ms() -> f64 {
    #[cfg(feature = "wasm")]
    {
        js_sys::Date::now()
    }
    #[cfg(not(feature = "wasm"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    }
}

pub(crate) fn log(msg: &str) {
    if !UPLOAD_PIPELINE_TRACE {
        return;
    }
    #[cfg(feature = "wasm")]
    {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(msg));
    }
    #[cfg(not(feature = "wasm"))]
    {
        eprintln!("{msg}");
    }
}

/// Hash/offload focused logs (intentionally not the full pipeline spam).
pub(crate) fn hash_log(msg: &str) {
    if !HASH_TRACE {
        return;
    }
    #[cfg(feature = "wasm")]
    {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(msg));
    }
    #[cfg(not(feature = "wasm"))]
    {
        eprintln!("{msg}");
    }
}
