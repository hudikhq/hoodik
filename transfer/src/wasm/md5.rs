//! Incremental MD5 for use in a dedicated hash Web Worker.
use digest::Digest;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransferMd5 {
    inner: md5::Md5,
}

#[wasm_bindgen]
impl TransferMd5 {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TransferMd5 {
        TransferMd5 {
            inner: md5::Md5::new(),
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

