pub mod base64;
pub mod error;
pub mod rsa;

pub use hex;
pub use rand;
pub use sha256;

mod utils;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn rsa_generate_private() -> Option<String> {
    utils::set_panic_hook();

    let private = rsa::private::generate().ok()?;

    rsa::private::to_string(&private).ok()
}
