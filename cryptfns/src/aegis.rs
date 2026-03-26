use crate::error::{CryptoResult, Error};
use ::aegis::aegis128l::Aegis128L;

const KEY_LENGTH: usize = 16;
const NONCE_LENGTH: usize = 16;
const TAG_LENGTH: usize = 16;

/// Generate random key + nonce for AEGIS-128L (32 bytes total, same layout as Ascon-128a).
pub fn generate_key() -> CryptoResult<Vec<u8>> {
    let mut key = vec![0u8; KEY_LENGTH + NONCE_LENGTH];
    getrandom::getrandom(&mut key)?;
    Ok(key)
}

/// Encrypt with AEGIS-128L.  Output = ciphertext ‖ 16-byte tag.
pub fn encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (k, n) = split_key_nonce(key);
    let (ct, tag) = Aegis128L::<TAG_LENGTH>::new(
        k.as_slice().try_into().map_err(|_| Error::UnknownCipher("bad key length".into()))?,
        n.as_slice().try_into().map_err(|_| Error::UnknownCipher("bad nonce length".into()))?,
    )
    .encrypt(&plaintext, &[]);

    let mut out = ct;
    out.extend_from_slice(&tag);
    Ok(out)
}

/// Decrypt with AEGIS-128L.  Expects ciphertext ‖ 16-byte tag as produced by [`encrypt`].
pub fn decrypt(key: Vec<u8>, ciphertext_with_tag: Vec<u8>) -> CryptoResult<Vec<u8>> {
    if ciphertext_with_tag.len() < TAG_LENGTH {
        return Err(Error::UnknownCipher("ciphertext too short".into()));
    }

    let (k, n) = split_key_nonce(key);
    let split = ciphertext_with_tag.len() - TAG_LENGTH;
    let (ct, tag_bytes) = ciphertext_with_tag.split_at(split);
    let tag: &[u8; TAG_LENGTH] = tag_bytes
        .try_into()
        .map_err(|_| Error::UnknownCipher("bad tag length".into()))?;

    Aegis128L::<TAG_LENGTH>::new(
        k.as_slice().try_into().map_err(|_| Error::UnknownCipher("bad key length".into()))?,
        n.as_slice().try_into().map_err(|_| Error::UnknownCipher("bad nonce length".into()))?,
    )
    .decrypt(ct, tag, &[])
    .map_err(|_| Error::UnknownCipher("aegis128l decryption failed".into()))
}

fn split_key_nonce(mut input: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let total = KEY_LENGTH + NONCE_LENGTH;
    match input.len().cmp(&total) {
        std::cmp::Ordering::Greater => input.truncate(total),
        std::cmp::Ordering::Less => input.resize(total, 0),
        std::cmp::Ordering::Equal => {}
    }
    let key = input[..KEY_LENGTH].to_vec();
    let nonce = input[KEY_LENGTH..].to_vec();
    (key, nonce)
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        let key = super::generate_key().unwrap();
        let plaintext = b"hello aegis-128l".to_vec();
        let ct = super::encrypt(key.clone(), plaintext.clone()).unwrap();
        let pt = super::decrypt(key, ct).unwrap();
        assert_eq!(plaintext, pt);
    }

    #[test]
    fn wrong_key_fails() {
        let key1 = super::generate_key().unwrap();
        let key2 = super::generate_key().unwrap();
        let ct = super::encrypt(key1, b"data".to_vec()).unwrap();
        assert!(super::decrypt(key2, ct).is_err());
    }
}
