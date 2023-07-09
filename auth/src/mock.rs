use crate::providers::signature::SignatureProvider;

/// Shortcut to access the non public API for integration testing
pub fn generate_fingerprint_nonce(fingerprint: &str) -> String {
    SignatureProvider::generate_fingerprint_nonce(fingerprint)
}
