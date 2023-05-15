use crate::error::{CryptoResult, Error};
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, Key, KeyInit, Nonce};

const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 12;

/// Generate random key with nonce for encryption/decryption
pub fn generate_key() -> CryptoResult<Vec<u8>> {
    let mut random_key = vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    getrandom::getrandom(&mut random_key)?;

    Ok(random_key)
}

/// Simple encrypt of the plaintext using chacha-poly1305
pub fn encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (key, nonce) = split_key_nonce(key.clone());
    let key = Key::from_slice(&key);
    let nonce = Nonce::from_slice(&nonce);
    let cipher = ChaCha20Poly1305::new(&key);

    cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(Error::from)
}

/// Simple decrypt of the ciphertext using chacha-poly1305
pub fn decrypt(key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (key, nonce) = split_key_nonce(key.clone());
    let key = Key::from_slice(&key);
    let nonce = Nonce::from_slice(&nonce);
    let cipher = ChaCha20Poly1305::new(&key);

    cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(Error::from)
}

/// Split the key and nonce from the input
pub fn split_key_nonce(mut input: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let input_length = input.len();

    match input_length > KEY_LENGTH + NONCE_LENGTH {
        true => {
            input.truncate(KEY_LENGTH + NONCE_LENGTH);
        }
        false => {
            input.resize(KEY_LENGTH + NONCE_LENGTH, 0);
        }
    };

    let key = input[..KEY_LENGTH].to_vec();
    let nonce = input[KEY_LENGTH..].to_vec();

    (key, nonce)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_chacha_encrypt_and_decrypt() {
        let plaintext = b"plaintext message".to_vec();
        let key = b"very secret key.very secret key.".to_vec();

        let encrypted = super::encrypt(key.clone(), plaintext.clone()).unwrap();

        let decrypted = super::decrypt(key, encrypted.clone()).unwrap();

        assert_eq!(
            String::from_utf8(plaintext).unwrap(),
            String::from_utf8(decrypted).unwrap()
        );
    }
}
