//! Incremental BLAKE2b-512 for use in a dedicated hash Web Worker.
use digest::Digest;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct TransferBlake2b512 {
    inner: blake2::Blake2b512,
}

#[wasm_bindgen]
impl TransferBlake2b512 {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TransferBlake2b512 {
        TransferBlake2b512 {
            inner: blake2::Blake2b512::new(),
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

