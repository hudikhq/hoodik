/// Shortcut to access the non public API for integration testing
pub fn generate_fingerprint_nonce(fingerprint: &str) -> String {
    crate::auth::Auth::generate_fingerprint_nonce(fingerprint)
}
