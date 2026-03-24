//! Incremental SHA-256 for use in a dedicated hash Web Worker (second WASM instance).

use digest::Digest;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransferSha256 {
    inner: sha2::Sha256,
}

#[wasm_bindgen]
impl TransferSha256 {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TransferSha256 {
        TransferSha256 {
            inner: sha2::Sha256::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    #[wasm_bindgen(js_name = finalizeHex)]
    pub fn finalize_hex(self) -> String {
        hex::encode(self.inner.finalize())
    }
}
