use crate::{aegis, aes, chacha};
use crate::error::{CryptoResult, Error};

/// Default cipher identifier — used when no cipher is specified.
pub const DEFAULT: &str = "aegis128l";

/// Symmetric cipher selector.
///
/// Each variant delegates to its underlying crate in `cryptfns`.
/// All key material is opaque `Vec<u8>` (key ‖ nonce concatenated):
/// - Ascon-128a / AEGIS-128L: 16-byte key + 16-byte nonce = 32 bytes
/// - ChaCha20-Poly1305: 32-byte key + 12-byte nonce = 44 bytes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cipher {
    Ascon128a,
    ChaCha20Poly1305,
    Aegis128L,
}

impl std::str::FromStr for Cipher {
    type Err = Error;

    /// Parse a cipher identifier string.  Empty string and `"ascon128a"` both map to the
    /// default [`Cipher::Ascon128a`] for backward-compatibility with existing data.
    fn from_str(s: &str) -> CryptoResult<Self> {
        match s {
            "ascon128a" | "" => Ok(Self::Ascon128a),
            "chacha20poly1305" => Ok(Self::ChaCha20Poly1305),
            "aegis128l" => Ok(Self::Aegis128L),
            other => Err(Error::UnknownCipher(other.to_string())),
        }
    }
}

impl Cipher {

    /// Return the canonical string identifier for this cipher.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ascon128a => "ascon128a",
            Self::ChaCha20Poly1305 => "chacha20poly1305",
            Self::Aegis128L => "aegis128l",
        }
    }

    /// Generate a fresh random key (with embedded nonce) for this cipher.
    pub fn generate_key(&self) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::generate_key(),
            Self::ChaCha20Poly1305 => chacha::generate_key(),
            Self::Aegis128L => aegis::generate_key(),
        }
    }

    /// Encrypt `plaintext` with `key`.  The key format is cipher-specific (see module docs).
    pub fn encrypt(&self, key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::encrypt(key, plaintext),
            Self::ChaCha20Poly1305 => chacha::encrypt(key, plaintext),
            Self::Aegis128L => aegis::encrypt(key, plaintext),
        }
    }

    /// Decrypt `ciphertext` with `key`.
    pub fn decrypt(&self, key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::decrypt(key, ciphertext),
            Self::ChaCha20Poly1305 => chacha::decrypt(key, ciphertext),
            Self::Aegis128L => aegis::decrypt(key, ciphertext),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn from_str_defaults() {
        assert_eq!(Cipher::from_str("ascon128a").unwrap(), Cipher::Ascon128a);
        assert_eq!(Cipher::from_str("").unwrap(), Cipher::Ascon128a);
        assert_eq!(Cipher::from_str("chacha20poly1305").unwrap(), Cipher::ChaCha20Poly1305);
        assert_eq!(Cipher::from_str("aegis128l").unwrap(), Cipher::Aegis128L);
    }

    #[test]
    fn from_str_unknown_errors() {
        assert!(Cipher::from_str("aes256gcm").is_err());
    }

    #[test]
    fn as_str_round_trips() {
        for cipher in [Cipher::Ascon128a, Cipher::ChaCha20Poly1305, Cipher::Aegis128L] {
            assert_eq!(Cipher::from_str(cipher.as_str()).unwrap(), cipher);
        }
    }

    #[test]
    fn ascon_encrypt_decrypt() {
        let cipher = Cipher::Ascon128a;
        let key = cipher.generate_key().unwrap();
        let plaintext = b"hello ascon".to_vec();
        let ciphertext = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
        let recovered = cipher.decrypt(key, ciphertext).unwrap();
        assert_eq!(plaintext, recovered);
    }

    #[test]
    fn chacha_encrypt_decrypt() {
        let cipher = Cipher::ChaCha20Poly1305;
        let key = cipher.generate_key().unwrap();
        let plaintext = b"hello chacha".to_vec();
        let ciphertext = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
        let recovered = cipher.decrypt(key, ciphertext).unwrap();
        assert_eq!(plaintext, recovered);
    }

    #[test]
    fn aegis_encrypt_decrypt() {
        let cipher = Cipher::Aegis128L;
        let key = cipher.generate_key().unwrap();
        let plaintext = b"hello aegis-128l".to_vec();
        let ciphertext = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
        let recovered = cipher.decrypt(key, ciphertext).unwrap();
        assert_eq!(plaintext, recovered);
    }

    #[test]
    fn ciphers_are_not_interchangeable() {
        let ascon = Cipher::Ascon128a;
        let chacha = Cipher::ChaCha20Poly1305;
        // Use an ascon key (32 bytes) — chacha expects 44 bytes, so decrypt should fail
        let key = ascon.generate_key().unwrap();
        let plaintext = b"cross-cipher".to_vec();
        let ciphertext = ascon.encrypt(key.clone(), plaintext).unwrap();
        assert!(chacha.decrypt(key, ciphertext).is_err());
    }
}
