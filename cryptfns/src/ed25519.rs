use crate::error::{CryptoResult, Error};

use ed25519_dalek::pkcs8::{
    spki::der::pem::LineEnding, DecodePrivateKey, DecodePublicKey, EncodePrivateKey,
    EncodePublicKey,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

/// Fingerprint of an Ed25519 public key — the key-type-agnostic SPKI
/// derivation, not the legacy RSA modulus hash.
pub fn fingerprint(public_key_pem: &str) -> CryptoResult<String> {
    crate::spki::fingerprint(public_key_pem)
}

pub mod private {
    use super::*;

    /// Generate a new signing key as a PKCS#8 PEM string.
    pub fn generate() -> CryptoResult<String> {
        let key = SigningKey::generate(&mut rand::rngs::OsRng);
        let pem = key
            .to_pkcs8_pem(LineEnding::LF)
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;

        Ok(pem.to_string())
    }

    /// Sign raw bytes, returning the signature as base64.
    pub fn sign_bytes(message: &[u8], key: &str) -> CryptoResult<String> {
        let key = from_str(key)?;
        let signature = key.sign(message);

        Ok(crate::base64::encode(signature.to_bytes()))
    }

    /// Sign a UTF-8 string, returning the signature as base64.
    pub fn sign(message: &str, key: &str) -> CryptoResult<String> {
        sign_bytes(message.as_bytes(), key)
    }

    pub fn from_str(input: &str) -> CryptoResult<SigningKey> {
        SigningKey::from_pkcs8_pem(input).map_err(|e| Error::KeyEncoding(e.to_string()))
    }
}

pub mod public {
    use super::*;

    /// Derive the public key (SPKI PEM) from a PKCS#8 PEM private key.
    pub fn from_private(key: &str) -> CryptoResult<String> {
        let key = private::from_str(key)?;
        let pem = key
            .verifying_key()
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;

        Ok(pem)
    }

    /// Verify a base64 signature over raw bytes against an SPKI PEM public key.
    pub fn verify_bytes(message: &[u8], signature: &str, key: &str) -> CryptoResult<()> {
        let key = from_str(key)?;
        let signature = crate::base64::decode(signature)?;
        let signature = Signature::from_slice(&signature)?;

        key.verify(message, &signature)?;

        Ok(())
    }

    /// Verify a base64 signature over a UTF-8 string.
    pub fn verify(message: &str, signature: &str, key: &str) -> CryptoResult<()> {
        verify_bytes(message.as_bytes(), signature, key)
    }

    pub fn from_str(input: &str) -> CryptoResult<VerifyingKey> {
        VerifyingKey::from_public_key_pem(input).map_err(|e| Error::KeyEncoding(e.to_string()))
    }

    /// Rebuild the SPKI PEM from stored SPKI DER so a superseded Ed25519 key
    /// can be re-loaded for verification (curve → curve rotation).
    pub fn pem_from_spki_der(der: &[u8]) -> CryptoResult<String> {
        VerifyingKey::from_public_key_der(der)
            .map_err(|e| Error::KeyEncoding(e.to_string()))?
            .to_public_key_pem(LineEnding::LF)
            .map_err(|e| Error::KeyEncoding(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sign_verify_round_trip() {
        let key = super::private::generate().unwrap();
        let public = super::public::from_private(&key).unwrap();

        let signature = super::private::sign("hello ed25519", &key).unwrap();
        super::public::verify("hello ed25519", &signature, &public).unwrap();
    }

    #[test]
    fn tampered_message_fails() {
        let key = super::private::generate().unwrap();
        let public = super::public::from_private(&key).unwrap();

        let signature = super::private::sign("original", &key).unwrap();
        assert!(super::public::verify("tampered", &signature, &public).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let key = super::private::generate().unwrap();
        let other = super::private::generate().unwrap();
        let other_public = super::public::from_private(&other).unwrap();

        let signature = super::private::sign("message", &key).unwrap();
        assert!(super::public::verify("message", &signature, &other_public).is_err());
    }

    #[test]
    fn pem_round_trips_through_parse() {
        let key = super::private::generate().unwrap();
        let public = super::public::from_private(&key).unwrap();

        assert!(key.contains("BEGIN PRIVATE KEY"));
        assert!(public.contains("BEGIN PUBLIC KEY"));

        let signature = super::private::sign_bytes(b"bytes", &key).unwrap();
        super::public::verify_bytes(b"bytes", &signature, &public).unwrap();
    }
}
