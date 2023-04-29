use crate::aes;
use crate::rsa;
use crate::rsa::PublicKeyParts;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn rsa_generate_private() -> Option<String> {
    crate::utils::set_panic_hook();

    let private = rsa::private::generate().ok()?;

    rsa::private::to_string(&private).ok()
}

#[wasm_bindgen]
pub fn rsa_private_key_size(private_key: String) -> Option<usize> {
    crate::utils::set_panic_hook();

    let private = rsa::private::from_str(&private_key).ok()?;

    Some(private.size() * 8)
}

#[wasm_bindgen]
pub fn rsa_public_key_size(public_key: String) -> Option<usize> {
    crate::utils::set_panic_hook();

    let public = rsa::public::from_str(&public_key).ok()?;

    Some(public.size() * 8)
}

#[wasm_bindgen]
pub fn rsa_public_from_private(private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    let private = rsa::private::from_str(&private_key).ok()?;
    let public = rsa::public::from_private(&private).ok()?;

    rsa::public::to_string(&public).ok()
}

#[wasm_bindgen]
pub fn rsa_sign(message: String, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::private::sign(&message, &private_key).ok()
}

#[wasm_bindgen]
pub fn rsa_verify(message: String, signature: String, public_key: String) -> bool {
    crate::utils::set_panic_hook();

    rsa::public::verify(&message, &signature, &public_key).is_ok()
}

#[wasm_bindgen]
pub fn rsa_encrypt(message: String, public_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::public::encrypt(&message, &public_key).ok()
}

#[wasm_bindgen]
pub fn rsa_decrypt(message: String, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::private::decrypt(&message, &private_key).ok()
}

#[wasm_bindgen]
pub fn rsa_fingerprint_public(public_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::fingerprint(rsa::public::from_str(&public_key).ok()?).ok()
}

#[wasm_bindgen]
pub fn rsa_fingerprint_private(private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::fingerprint(rsa::private::from_str(&private_key).ok()?).ok()
}

#[wasm_bindgen]
pub fn aes_generate_key() -> Option<Vec<u8>> {
    aes::generate_key().ok()
}

#[wasm_bindgen]
pub fn aes_encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> Option<Vec<u8>> {
    aes::encrypt(key, plaintext).ok()
}

#[wasm_bindgen]
pub fn aes_decrypt(key: Vec<u8>, ciphertext: Vec<u8>) -> Option<Vec<u8>> {
    aes::decrypt(key, ciphertext).ok()
}

#[wasm_bindgen]
pub fn text_into_tokens(input: &str) -> Option<String> {
    crate::tokenizer::into_tokens(input)
        .ok()
        .map(crate::tokenizer::into_string)
}

#[wasm_bindgen]
pub fn text_into_hashed_tokens(input: &str) -> Option<String> {
    crate::tokenizer::into_tokens(input)
        .ok()
        .map(crate::tokenizer::into_string)
}
