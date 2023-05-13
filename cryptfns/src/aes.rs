use crate::error::{CryptoResult, Error};
use ascon_aead::aead::{Aead, KeyInit};
use ascon_aead::{Ascon128a, Key, Nonce};

const KEY_LENGTH: usize = 16;
const NONCE_LENGTH: usize = 16;

/// Generate random key with nonce for encryption/decryption
pub fn generate_key() -> CryptoResult<Vec<u8>> {
    let mut random_key = vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    getrandom::getrandom(&mut random_key)?;

    Ok(random_key)
}

/// Exposing the AES encrypt function
pub fn encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    encrypt_aead(key, plaintext)
}

/// Exposing the AES decrypt function
pub fn decrypt(key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    decrypt_aead(key, ciphertext)
}

/// Encrypt the data with given key and iv ASCON_AEAD
fn encrypt_aead(key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (key, nonce) = split_key_nonce(key);

    let key = Key::<Ascon128a>::from_slice(key.as_ref());
    let nonce = Nonce::<Ascon128a>::from_slice(nonce.as_ref());

    Ascon128a::new(key)
        .encrypt(nonce, plaintext.as_ref())
        .map_err(Error::from)
}

/// Decrypt the data with given key and iv ASCON_AEAD
fn decrypt_aead(key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (key, nonce) = split_key_nonce(key);

    let key = Key::<Ascon128a>::from_slice(key.as_ref());
    let nonce = Nonce::<Ascon128a>::from_slice(nonce.as_ref());

    Ascon128a::new(key)
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(Error::from)
}

/// Split the key and nonce from the input
fn split_key_nonce(mut input: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
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
    fn test_aes_encrypt_and_decrypt() {
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
