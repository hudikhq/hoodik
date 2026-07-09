//! X25519 file-key wrapping (ECIES construction).
//!
//! X25519 is key agreement only — there is no "encrypt to public key" like
//! RSA. Wrapping derives a one-off AEAD key instead:
//!
//! ```text
//! ephemeral = fresh X25519 keypair (per wrap)
//! shared    = X25519(ephemeral.private, recipient.public)
//! wrap_key  = HKDF-SHA256(ikm: shared, salt: ephemeral.pub ‖ recipient.pub, info: WRAP_INFO)
//! blob      = ephemeral.pub ‖ nonce ‖ AEGIS-256(wrap_key ‖ nonce, file_key)
//! ```
//!
//! Binding both public keys into the HKDF salt ties the derived key to this
//! exact pair, closing the unknown-key-share class. The output is one base64
//! blob so it stores in `user_files.encrypted_key` exactly like an RSA wrap.

use crate::error::{CryptoResult, Error};

use hkdf::Hkdf;
use pkcs8::der::asn1::BitString;
use pkcs8::der::pem::LineEnding;
use pkcs8::der::{Decode, Encode};
use pkcs8::spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned, SubjectPublicKeyInfoRef};
use pkcs8::{Document, ObjectIdentifier, PrivateKeyInfo};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};

/// id-X25519 from RFC 8410.
const X25519_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.110");

const KEY_LENGTH: usize = 32;
/// AEGIS-256 nonce — the wrap AEAD is fixed to AEGIS-256; the construction is
/// versioned through [`WRAP_INFO`], not negotiated per wrap.
const NONCE_LENGTH: usize = 32;
const TAG_LENGTH: usize = 16;

const WRAP_INFO: &[u8] = b"hoodik-file-key-wrap-v1";

/// Wrap a file key for a recipient's X25519 public key (SPKI PEM).
pub fn wrap(file_key: &[u8], recipient_public_key: &str) -> CryptoResult<String> {
    let recipient = public::from_str(recipient_public_key)?;

    let ephemeral = EphemeralSecret::random_from_rng(rand::rngs::OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral);

    let shared = ephemeral.diffie_hellman(&recipient);
    let wrap_key = derive_wrap_key(shared.as_bytes(), &ephemeral_public, &recipient)?;

    let mut nonce = vec![0u8; NONCE_LENGTH];
    getrandom::getrandom(&mut nonce)?;

    let mut aead_key = wrap_key.to_vec();
    aead_key.extend_from_slice(&nonce);
    let ciphertext = crate::aegis256::encrypt(aead_key, file_key.to_vec())?;

    let mut blob = ephemeral_public.as_bytes().to_vec();
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);

    Ok(crate::base64::encode(blob))
}

/// Unwrap a file key with the recipient's X25519 private key (PKCS#8 PEM).
pub fn unwrap(blob: &str, private_key: &str) -> CryptoResult<Vec<u8>> {
    let blob = crate::base64::decode(blob)?;
    if blob.len() < KEY_LENGTH + NONCE_LENGTH + TAG_LENGTH {
        return Err(Error::InvalidLength("ecdh wrap blob too short"));
    }

    let secret = private::from_str(private_key)?;
    let own_public = PublicKey::from(&secret);

    let ephemeral_public: [u8; KEY_LENGTH] = blob[..KEY_LENGTH]
        .try_into()
        .map_err(|_| Error::InvalidLength("bad ephemeral key length"))?;
    let ephemeral_public = PublicKey::from(ephemeral_public);
    let nonce = &blob[KEY_LENGTH..KEY_LENGTH + NONCE_LENGTH];
    let ciphertext = &blob[KEY_LENGTH + NONCE_LENGTH..];

    let shared = secret.diffie_hellman(&ephemeral_public);
    let wrap_key = derive_wrap_key(shared.as_bytes(), &ephemeral_public, &own_public)?;

    let mut aead_key = wrap_key.to_vec();
    aead_key.extend_from_slice(nonce);

    crate::aegis256::decrypt(aead_key, ciphertext.to_vec())
}

fn derive_wrap_key(
    shared: &[u8; KEY_LENGTH],
    ephemeral_public: &PublicKey,
    recipient_public: &PublicKey,
) -> CryptoResult<[u8; KEY_LENGTH]> {
    // An all-zero shared secret means a low-order/identity peer point —
    // the DH contributed nothing and the "secret" is attacker-known.
    if shared.iter().all(|b| *b == 0) {
        return Err(Error::KeyEncoding("non-contributory x25519 public key".to_string()));
    }

    let mut salt = ephemeral_public.as_bytes().to_vec();
    salt.extend_from_slice(recipient_public.as_bytes());

    let mut wrap_key = [0u8; KEY_LENGTH];
    Hkdf::<Sha256>::new(Some(&salt), shared)
        .expand(WRAP_INFO, &mut wrap_key)
        .map_err(|e| Error::KeyEncoding(e.to_string()))?;

    Ok(wrap_key)
}

pub mod private {
    use super::*;

    /// Generate a new X25519 private key as a PKCS#8 PEM string.
    pub fn generate() -> CryptoResult<String> {
        let secret = StaticSecret::random_from_rng(rand::rngs::OsRng);

        // RFC 8410: the PKCS#8 privateKey field holds CurvePrivateKey, an
        // OCTET STRING wrapping the raw 32 bytes.
        let inner = pkcs8::der::asn1::OctetString::new(secret.to_bytes().to_vec())
            .and_then(|s| s.to_der())
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;

        let info = PrivateKeyInfo {
            algorithm: pkcs8::AlgorithmIdentifierRef { oid: X25519_OID, parameters: None },
            private_key: &inner,
            public_key: None,
        };
        let der = info.to_der().map_err(|e| Error::KeyEncoding(e.to_string()))?;
        let document = Document::from_der(&der).map_err(|e| Error::KeyEncoding(e.to_string()))?;
        let pem = document
            .to_pem("PRIVATE KEY", LineEnding::LF)
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;

        Ok(pem)
    }

    pub fn from_str(input: &str) -> CryptoResult<StaticSecret> {
        let (label, document) =
            Document::from_pem(input).map_err(|e| Error::KeyEncoding(e.to_string()))?;
        if label != "PRIVATE KEY" {
            return Err(Error::KeyEncoding(format!("expected PRIVATE KEY pem, got {label}")));
        }

        let info = PrivateKeyInfo::from_der(document.as_bytes())
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;
        if info.algorithm.oid != X25519_OID {
            return Err(Error::KeyEncoding("not an x25519 key".to_string()));
        }

        let inner = pkcs8::der::asn1::OctetString::from_der(info.private_key)
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;
        let bytes: [u8; KEY_LENGTH] = inner
            .as_bytes()
            .try_into()
            .map_err(|_| Error::InvalidLength("bad x25519 private key length"))?;

        Ok(StaticSecret::from(bytes))
    }
}

pub mod public {
    use super::*;

    /// Derive the public key (SPKI PEM) from a PKCS#8 PEM private key.
    pub fn from_private(key: &str) -> CryptoResult<String> {
        let secret = private::from_str(key)?;
        let public = PublicKey::from(&secret);

        let spki = SubjectPublicKeyInfoOwned {
            algorithm: AlgorithmIdentifierOwned { oid: X25519_OID, parameters: None },
            subject_public_key: BitString::from_bytes(public.as_bytes())
                .map_err(|e| Error::KeyEncoding(e.to_string()))?,
        };
        let der = spki.to_der().map_err(|e| Error::KeyEncoding(e.to_string()))?;
        let document = Document::from_der(&der).map_err(|e| Error::KeyEncoding(e.to_string()))?;
        let pem = document
            .to_pem("PUBLIC KEY", LineEnding::LF)
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;

        Ok(pem)
    }

    pub fn from_str(input: &str) -> CryptoResult<PublicKey> {
        let (label, document) =
            Document::from_pem(input).map_err(|e| Error::KeyEncoding(e.to_string()))?;
        if label != "PUBLIC KEY" {
            return Err(Error::KeyEncoding(format!("expected PUBLIC KEY pem, got {label}")));
        }

        let spki = SubjectPublicKeyInfoRef::from_der(document.as_bytes())
            .map_err(|e| Error::KeyEncoding(e.to_string()))?;
        if spki.algorithm.oid != X25519_OID {
            return Err(Error::KeyEncoding("not an x25519 key".to_string()));
        }

        let bytes: [u8; KEY_LENGTH] = spki
            .subject_public_key
            .raw_bytes()
            .try_into()
            .map_err(|_| Error::InvalidLength("bad x25519 public key length"))?;

        Ok(PublicKey::from(bytes))
    }
}

#[cfg(test)]
mod tests {
    fn keypair() -> (String, String) {
        let private = super::private::generate().unwrap();
        let public = super::public::from_private(&private).unwrap();
        (private, public)
    }

    #[test]
    fn wrap_unwrap_round_trip() {
        let (private, public) = keypair();
        let file_key = crate::aegis256::generate_key().unwrap();

        let blob = super::wrap(&file_key, &public).unwrap();
        let recovered = super::unwrap(&blob, &private).unwrap();
        assert_eq!(file_key, recovered);
    }

    #[test]
    fn empty_and_large_and_multibyte_payloads() {
        let (private, public) = keypair();

        for payload in [
            Vec::new(),
            vec![0u8; 1024 * 64],
            "ključ-датотеке-鍵".as_bytes().to_vec(),
        ] {
            let blob = super::wrap(&payload, &public).unwrap();
            assert_eq!(super::unwrap(&blob, &private).unwrap(), payload);
        }
    }

    #[test]
    fn wrong_key_fails() {
        let (_, public) = keypair();
        let (other_private, _) = keypair();

        let blob = super::wrap(b"file-key", &public).unwrap();
        assert!(super::unwrap(&blob, &other_private).is_err());
    }

    #[test]
    fn ephemeral_is_unique_per_wrap() {
        let (_, public) = keypair();

        let a = super::wrap(b"same-key", &public).unwrap();
        let b = super::wrap(b"same-key", &public).unwrap();
        assert_ne!(a, b);

        let a = crate::base64::decode(&a).unwrap();
        let b = crate::base64::decode(&b).unwrap();
        assert_ne!(a[..32], b[..32]);
    }

    #[test]
    fn tampered_blob_fails() {
        let (private, public) = keypair();

        let blob = super::wrap(b"file-key", &public).unwrap();
        let mut bytes = crate::base64::decode(&blob).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;

        assert!(super::unwrap(&crate::base64::encode(bytes), &private).is_err());
    }

    #[test]
    fn truncated_blob_fails_cleanly() {
        let (private, _) = keypair();
        let short = crate::base64::encode(vec![0u8; 40]);
        assert!(super::unwrap(&short, &private).is_err());
    }

    #[test]
    fn pem_keys_parse_back() {
        let (private, public) = keypair();
        assert!(private.contains("BEGIN PRIVATE KEY"));
        assert!(public.contains("BEGIN PUBLIC KEY"));

        super::private::from_str(&private).unwrap();
        super::public::from_str(&public).unwrap();
    }
}
