use crate::error::{CryptoResult, Error};

/// Key-type-agnostic fingerprint: `SHA256(hex(SPKI DER))` of a PEM public key.
///
/// The legacy RSA fingerprint hashes the bare modulus (`rsa::fingerprint`) and
/// cannot be computed for non-RSA keys; this derivation works for any
/// algorithm because it hashes the whole SubjectPublicKeyInfo document, and it
/// is the go-forward scheme for Ed25519/X25519 identities.
pub fn fingerprint(public_key_pem: &str) -> CryptoResult<String> {
    let (label, document) = pkcs8::Document::from_pem(public_key_pem)
        .map_err(|e| Error::KeyEncoding(e.to_string()))?;

    if label != "PUBLIC KEY" {
        return Err(Error::KeyEncoding(format!("expected PUBLIC KEY pem, got {label}")));
    }

    Ok(sha256::digest(hex::encode(document.as_bytes())))
}

#[cfg(test)]
mod tests {
    #[test]
    fn fingerprint_is_deterministic_hex() {
        let key = crate::ed25519::private::generate().unwrap();
        let public = crate::ed25519::public::from_private(&key).unwrap();

        let a = super::fingerprint(&public).unwrap();
        let b = super::fingerprint(&public).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_keys_differ() {
        let a = crate::ed25519::public::from_private(&crate::ed25519::private::generate().unwrap())
            .unwrap();
        let b = crate::ed25519::public::from_private(&crate::ed25519::private::generate().unwrap())
            .unwrap();
        assert_ne!(super::fingerprint(&a).unwrap(), super::fingerprint(&b).unwrap());
    }

    #[test]
    fn rejects_private_key_pem() {
        let key = crate::ed25519::private::generate().unwrap();
        assert!(super::fingerprint(&key).is_err());
    }
}
