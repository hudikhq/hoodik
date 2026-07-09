//! Envelope encryption for the user's private-key bundle.
//!
//! The bundle (all of a user's private keys, opaque here) is sealed under a
//! random 256-bit data key, and only that data key is wrapped by the
//! password-derived KEK. A password change re-wraps the tiny data key
//! ([`rewrap`]) instead of re-encrypting the whole bundle, and the bundle
//! ciphertext never has to be decrypted to rotate the password.
//!
//! The KEK is derived from an OPAQUE `export_key` (or any high-entropy secret)
//! via [`derive_kek`]. The KEK is fixed per (user, password), so the data-key
//! wrap uses a fresh random nonce every seal — reusing a nonce under a fixed
//! AEGIS key would be catastrophic. The bundle wrap uses the data key itself
//! (fresh per seal) as key‖nonce, so it needs no separate nonce.
//!
//! Wire layout (then base64), all AEADs AEGIS-256:
//! ```text
//! version: u8 = 1
//! kek_nonce: [u8; 32]              random, nonce for the data-key wrap
//! wrapped_data_key_len: u16 BE
//! wrapped_data_key: [..]           AEGIS256(kek ‖ kek_nonce, data_key)
//! ciphertext: [..]                 AEGIS256(data_key, bundle)
//! ```

use crate::error::{CryptoResult, Error};

use hkdf::Hkdf;
use sha2::Sha512;

const VERSION: u8 = 1;
const KEK_LENGTH: usize = 32;
const KEK_NONCE_LENGTH: usize = 32;
/// AEGIS-256 key‖nonce blob length (32-byte key + 32-byte nonce).
const DATA_KEY_LENGTH: usize = 64;

const KEK_INFO: &[u8] = b"hoodik-private-key-kek-v1";

/// Derive the 256-bit key-encryption key from an OPAQUE `export_key` (or any
/// high-entropy input). Domain-separated so the same `export_key` can seed
/// other purposes under different labels without colliding.
pub fn derive_kek(export_key: &[u8]) -> CryptoResult<[u8; KEK_LENGTH]> {
    let mut kek = [0u8; KEK_LENGTH];
    Hkdf::<Sha512>::new(None, export_key)
        .expand(KEK_INFO, &mut kek)
        .map_err(|e| Error::KeyEncoding(e.to_string()))?;

    Ok(kek)
}

/// Seal a private-key bundle: wrap it under a fresh random data key, wrap that
/// data key under `kek`, and return the base64 envelope.
pub fn seal(kek: &[u8; KEK_LENGTH], bundle: &[u8]) -> CryptoResult<String> {
    let data_key = crate::aegis256::generate_key()?;

    let mut kek_nonce = vec![0u8; KEK_NONCE_LENGTH];
    getrandom::getrandom(&mut kek_nonce)?;

    let wrapped_data_key = crate::aegis256::encrypt(wrap_key(kek, &kek_nonce), data_key.clone())?;
    let ciphertext = crate::aegis256::encrypt(data_key, bundle.to_vec())?;

    Ok(crate::base64::encode(assemble(&kek_nonce, &wrapped_data_key, &ciphertext)))
}

/// Open an envelope: recover the data key with `kek`, then the bundle.
pub fn open(kek: &[u8; KEK_LENGTH], envelope: &str) -> CryptoResult<Vec<u8>> {
    let blob = crate::base64::decode(envelope)?;
    let (kek_nonce, wrapped_data_key, ciphertext) = parse(&blob)?;

    let data_key = crate::aegis256::decrypt(wrap_key(kek, kek_nonce), wrapped_data_key.to_vec())?;
    if data_key.len() != DATA_KEY_LENGTH {
        return Err(Error::InvalidLength("envelope data key wrong length"));
    }

    crate::aegis256::decrypt(data_key, ciphertext.to_vec())
}

/// Re-wrap the data key under a new KEK without touching the bundle
/// ciphertext — the cheap half of a password change.
pub fn rewrap(
    old_kek: &[u8; KEK_LENGTH],
    new_kek: &[u8; KEK_LENGTH],
    envelope: &str,
) -> CryptoResult<String> {
    let blob = crate::base64::decode(envelope)?;
    let (kek_nonce, wrapped_data_key, ciphertext) = parse(&blob)?;

    let data_key = crate::aegis256::decrypt(wrap_key(old_kek, kek_nonce), wrapped_data_key.to_vec())?;
    if data_key.len() != DATA_KEY_LENGTH {
        return Err(Error::InvalidLength("envelope data key wrong length"));
    }

    let mut new_nonce = vec![0u8; KEK_NONCE_LENGTH];
    getrandom::getrandom(&mut new_nonce)?;
    let new_wrapped_data_key = crate::aegis256::encrypt(wrap_key(new_kek, &new_nonce), data_key)?;

    Ok(crate::base64::encode(assemble(&new_nonce, &new_wrapped_data_key, ciphertext)))
}

fn wrap_key(kek: &[u8; KEK_LENGTH], nonce: &[u8]) -> Vec<u8> {
    let mut key = kek.to_vec();
    key.extend_from_slice(nonce);
    key
}

fn assemble(kek_nonce: &[u8], wrapped_data_key: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 + KEK_NONCE_LENGTH + 2 + wrapped_data_key.len() + ciphertext.len());
    out.push(VERSION);
    out.extend_from_slice(kek_nonce);
    out.extend_from_slice(&(wrapped_data_key.len() as u16).to_be_bytes());
    out.extend_from_slice(wrapped_data_key);
    out.extend_from_slice(ciphertext);
    out
}

fn parse(blob: &[u8]) -> CryptoResult<(&[u8], &[u8], &[u8])> {
    let header = 1 + KEK_NONCE_LENGTH + 2;
    if blob.len() < header {
        return Err(Error::InvalidLength("envelope too short"));
    }
    if blob[0] != VERSION {
        return Err(Error::KeyEncoding(format!("unsupported envelope version {}", blob[0])));
    }

    let kek_nonce = &blob[1..1 + KEK_NONCE_LENGTH];
    let len = u16::from_be_bytes([blob[1 + KEK_NONCE_LENGTH], blob[2 + KEK_NONCE_LENGTH]]) as usize;
    let wrapped_end = header + len;
    if blob.len() < wrapped_end {
        return Err(Error::InvalidLength("envelope wrapped data key truncated"));
    }

    Ok((kek_nonce, &blob[header..wrapped_end], &blob[wrapped_end..]))
}

#[cfg(test)]
mod tests {
    fn kek(seed: u8) -> [u8; super::KEK_LENGTH] {
        super::derive_kek(&[seed; 48]).unwrap()
    }

    #[test]
    fn seal_open_round_trip() {
        let kek = kek(1);
        let bundle = b"rsa-pem||ed25519-pem||x25519-pem".to_vec();

        let envelope = super::seal(&kek, &bundle).unwrap();
        assert_eq!(super::open(&kek, &envelope).unwrap(), bundle);
    }

    #[test]
    fn empty_and_large_bundles() {
        let kek = kek(2);
        for bundle in [Vec::new(), vec![7u8; 8192]] {
            let envelope = super::seal(&kek, &bundle).unwrap();
            assert_eq!(super::open(&kek, &envelope).unwrap(), bundle);
        }
    }

    #[test]
    fn wrong_kek_fails() {
        let bundle = b"secret".to_vec();
        let envelope = super::seal(&kek(3), &bundle).unwrap();
        assert!(super::open(&kek(4), &envelope).is_err());
    }

    #[test]
    fn two_seals_reuse_no_nonce() {
        let kek = kek(5);
        let a = crate::base64::decode(&super::seal(&kek, b"same").unwrap()).unwrap();
        let b = crate::base64::decode(&super::seal(&kek, b"same").unwrap()).unwrap();
        // kek_nonce and the fresh data key both differ, so nothing repeats.
        assert_ne!(a[1..1 + super::KEK_NONCE_LENGTH], b[1..1 + super::KEK_NONCE_LENGTH]);
        assert_ne!(a, b);
    }

    #[test]
    fn rewrap_preserves_bundle_and_switches_kek() {
        let old = kek(6);
        let new = kek(7);
        let bundle = b"private-keys".to_vec();

        let envelope = super::seal(&old, &bundle).unwrap();
        let rewrapped = super::rewrap(&old, &new, &envelope).unwrap();

        assert_eq!(super::open(&new, &rewrapped).unwrap(), bundle);
        assert!(super::open(&old, &rewrapped).is_err());
    }

    #[test]
    fn rewrap_leaves_bundle_ciphertext_untouched() {
        let old = kek(8);
        let new = kek(9);

        let envelope = super::seal(&old, b"unchanging-bundle").unwrap();
        let a = crate::base64::decode(&envelope).unwrap();
        let rewrapped = super::rewrap(&old, &new, &envelope).unwrap();
        let b = crate::base64::decode(&rewrapped).unwrap();

        // The bundle ciphertext (everything past the wrapped data key) is
        // byte-identical — only the wrapped data key changed.
        let (_, wa, ca) = super::parse(&a).unwrap();
        let (_, wb, cb) = super::parse(&b).unwrap();
        assert_eq!(ca, cb);
        assert_ne!(wa, wb);
    }

    #[test]
    fn derive_kek_is_deterministic() {
        assert_eq!(super::derive_kek(b"export").unwrap(), super::derive_kek(b"export").unwrap());
        assert_ne!(super::derive_kek(b"a").unwrap(), super::derive_kek(b"b").unwrap());
    }

    #[test]
    fn tampered_and_truncated_fail() {
        let kek = kek(10);
        let envelope = super::seal(&kek, b"data").unwrap();
        let mut bytes = crate::base64::decode(&envelope).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        assert!(super::open(&kek, &crate::base64::encode(bytes)).is_err());

        assert!(super::open(&kek, &crate::base64::encode(vec![super::VERSION; 4])).is_err());
    }
}
