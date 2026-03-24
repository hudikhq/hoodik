//! Incremental SHA-1 for use in a dedicated hash Web Worker.
use digest::Digest;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransferSha1 {
    inner: sha1::Sha1,
}

#[wasm_bindgen]
impl TransferSha1 {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TransferSha1 {
        TransferSha1 {
            inner: sha1::Sha1::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.inner.update(data);
    }

    #[wasm_bindgen(js_name = finalizeHex)]
    pub fn finalize_hex(self) -> String {
        hex::encode(self.inner.finalize().as_slice())
    }
}

