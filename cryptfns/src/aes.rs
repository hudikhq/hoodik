use crate::error::{CryptoResult, Error};
use ascon_aead::aead::{Aead, KeyInit};
use ascon_aead::{Ascon128a, Key, Nonce};

const KEY_LENGTH: usize = 16;
const NONCE_LENGTH: usize = 16;

/// 32 random bytes: first 16 are the Ascon-128a key, last 16 are the nonce.
/// Callers pass the whole slice around as a single "key"; [`split_key_nonce`]
/// splits it at the boundary at encrypt/decrypt time.
pub fn generate_key() -> CryptoResult<Vec<u8>> {
    let mut random_key = vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    getrandom::getrandom(&mut random_key)?;

    Ok(random_key)
}

pub fn encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    encrypt_aead(key, plaintext)
}

pub fn decrypt(key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    decrypt_aead(key, ciphertext)
}

fn encrypt_aead(key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (key, nonce) = split_key_nonce(key);

    let key = Key::<Ascon128a>::from_slice(key.as_ref());
    let nonce = Nonce::<Ascon128a>::from_slice(nonce.as_ref());

    Ascon128a::new(key)
        .encrypt(nonce, plaintext.as_ref())
        .map_err(Error::from)
}

fn decrypt_aead(key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
    let (key, nonce) = split_key_nonce(key);

    let key = Key::<Ascon128a>::from_slice(key.as_ref());
    let nonce = Nonce::<Ascon128a>::from_slice(nonce.as_ref());

    Ascon128a::new(key)
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(Error::from)
}

/// Pads or truncates to exactly `KEY_LENGTH + NONCE_LENGTH` bytes before
/// splitting, so short inputs don't panic and long ones don't leak.
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

    #[test]
    fn fails_decrypt_with_wrong_key() {
        let plaintext = b"plaintext message".to_vec();
        let right_key = b"very secret key.very secret key.".to_vec();
        let wrong_key = b"very wrong  key.very wrong  key.".to_vec();

        let encrypted = super::encrypt(right_key, plaintext).unwrap();

        let result = super::decrypt(wrong_key, encrypted);

        assert!(result.is_err())
    }
    #[test]
    fn test_aes_encrypt_and_decrypt_characters() {
        let non_ascii = "あいうえお";

        let plaintext = non_ascii.as_bytes().to_vec();
        let key = b"very secret key.very secret key.".to_vec();

        let encrypted = super::encrypt(key.clone(), plaintext.clone()).unwrap();

        let decrypted = super::decrypt(key, encrypted.clone()).unwrap();

        assert_eq!(
            non_ascii,
            &String::from_utf8(decrypted).unwrap()
        );
    }
}
