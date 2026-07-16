use crate::{aegis, aegis256, aes, chacha};
use crate::error::{CryptoResult, Error};

/// Default cipher identifier — used when no cipher is specified.
pub const DEFAULT: &str = "aegis128l";

/// Symmetric cipher selector.
///
/// Each variant delegates to its underlying crate in `cryptfns`.
/// All key material is opaque `Vec<u8>` (key ‖ nonce concatenated):
/// - Ascon-128a / AEGIS-128L: 16-byte key + 16-byte nonce = 32 bytes
/// - ChaCha20-Poly1305: 32-byte key + 12-byte nonce = 44 bytes
/// - AEGIS-256: 32-byte key + 32-byte nonce = 64 bytes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cipher {
    Ascon128a,
    ChaCha20Poly1305,
    Aegis128L,
    Aegis256,
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
            "aegis256" => Ok(Self::Aegis256),
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
            Self::Aegis256 => "aegis256",
        }
    }

    /// Generate a fresh random key (with embedded nonce) for this cipher.
    pub fn generate_key(&self) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::generate_key(),
            Self::ChaCha20Poly1305 => chacha::generate_key(),
            Self::Aegis128L => aegis::generate_key(),
            Self::Aegis256 => aegis256::generate_key(),
        }
    }

    /// Encrypt `plaintext` with `key`.  The key format is cipher-specific (see module docs).
    pub fn encrypt(&self, key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::encrypt(key, plaintext),
            Self::ChaCha20Poly1305 => chacha::encrypt(key, plaintext),
            Self::Aegis128L => aegis::encrypt(key, plaintext),
            Self::Aegis256 => aegis256::encrypt(key, plaintext),
        }
    }

    /// Decrypt `ciphertext` with `key`.
    pub fn decrypt(&self, key: Vec<u8>, ciphertext: Vec<u8>) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::decrypt(key, ciphertext),
            Self::ChaCha20Poly1305 => chacha::decrypt(key, ciphertext),
            Self::Aegis128L => aegis::decrypt(key, ciphertext),
            Self::Aegis256 => aegis256::decrypt(key, ciphertext),
        }
    }

    /// Encrypt one chunk of a multi-chunk payload.
    ///
    /// Encrypting every chunk with the blob as-is would reuse the embedded nonce
    /// across all chunks of a file, voiding the AEAD security guarantees — for
    /// ChaCha20-Poly1305 the XOR of two ciphertext chunks directly leaks the XOR
    /// of their plaintexts.  Each chunk therefore gets its own nonce, derived by
    /// XOR-ing the little-endian chunk index into the nonce portion of the blob.
    ///
    /// Index 0 leaves the blob unchanged, so single-chunk payloads are
    /// byte-identical to [`Cipher::encrypt`] output.
    pub fn encrypt_chunk(
        &self,
        key: &[u8],
        chunk_index: u64,
        plaintext: Vec<u8>,
    ) -> CryptoResult<Vec<u8>> {
        self.encrypt(self.chunk_key(key, chunk_index), plaintext)
    }

    /// Decrypt one chunk of a multi-chunk payload.
    ///
    /// Tries the per-chunk nonce first (the [`Cipher::encrypt_chunk`] scheme).
    /// Files uploaded before per-chunk nonces existed encrypted every chunk with
    /// the blob's nonce as-is, so on failure the unmodified blob is tried as a
    /// fallback.  The AEAD tag rejects the wrong branch, so the fallback cannot
    /// misdecrypt.
    pub fn decrypt_chunk(
        &self,
        key: &[u8],
        chunk_index: u64,
        ciphertext: Vec<u8>,
    ) -> CryptoResult<Vec<u8>> {
        if chunk_index == 0 {
            return self.decrypt(key.to_vec(), ciphertext);
        }
        self.decrypt(self.chunk_key(key, chunk_index), ciphertext.clone())
            .or_else(|_| self.decrypt(key.to_vec(), ciphertext))
    }

    /// Encrypt a metadata string (file name, thumbnail, link fields).
    ///
    /// Metadata strings share the file key with the content chunks and with
    /// each other — under the key's embedded nonce they would all reuse the
    /// same (key, nonce) pair.  Each string therefore gets a fresh random
    /// nonce, prepended to the ciphertext so [`Cipher::decrypt_string`] can
    /// recover it.
    pub fn encrypt_string(&self, key: Vec<u8>, plaintext: Vec<u8>) -> CryptoResult<Vec<u8>> {
        let (key_len, nonce_len) = self.key_nonce_lengths();
        let mut nonce = vec![0u8; nonce_len];
        getrandom::getrandom(&mut nonce)?;

        let mut blob = key;
        blob.resize(key_len, 0);
        blob.extend_from_slice(&nonce);

        let ciphertext = self.encrypt(blob, plaintext)?;
        let mut out = nonce;
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt a metadata string.
    ///
    /// Tries the random-nonce format first (nonce prepended by
    /// [`Cipher::encrypt_string`]), then falls back to the legacy layout that
    /// used the key's embedded nonce — the AEAD tag rejects whichever branch
    /// is wrong, so the fallback cannot misdecrypt.
    pub fn decrypt_string(&self, key: Vec<u8>, data: Vec<u8>) -> CryptoResult<Vec<u8>> {
        let (key_len, nonce_len) = self.key_nonce_lengths();

        if data.len() > nonce_len {
            let mut blob = key.clone();
            blob.resize(key_len, 0);
            blob.extend_from_slice(&data[..nonce_len]);

            if let Ok(plaintext) = self.decrypt(blob, data[nonce_len..].to_vec()) {
                return Ok(plaintext);
            }
        }

        self.decrypt(key, data)
    }

    /// Derive the key blob for `chunk_index`: XOR the little-endian index into
    /// the first 8 bytes of the embedded nonce, leaving the key bytes untouched.
    /// Normalises the blob length first, matching each cipher's `split_key_nonce`.
    fn chunk_key(&self, key: &[u8], chunk_index: u64) -> Vec<u8> {
        let (key_len, nonce_len) = self.key_nonce_lengths();
        let mut blob = key.to_vec();
        blob.resize(key_len + nonce_len, 0);
        for (i, b) in chunk_index.to_le_bytes().iter().enumerate() {
            blob[key_len + i] ^= b;
        }
        blob
    }

    fn key_nonce_lengths(&self) -> (usize, usize) {
        match self {
            Self::Ascon128a => (aes::KEY_LENGTH, aes::NONCE_LENGTH),
            Self::ChaCha20Poly1305 => (chacha::KEY_LENGTH, chacha::NONCE_LENGTH),
            Self::Aegis128L => (aegis::KEY_LENGTH, aegis::NONCE_LENGTH),
            Self::Aegis256 => (aegis256::KEY_LENGTH, aegis256::NONCE_LENGTH),
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
        assert_eq!(Cipher::from_str("aegis256").unwrap(), Cipher::Aegis256);
    }

    #[test]
    fn from_str_unknown_errors() {
        assert!(Cipher::from_str("aes256gcm").is_err());
    }

    #[test]
    fn as_str_round_trips() {
        for cipher in [
            Cipher::Ascon128a,
            Cipher::ChaCha20Poly1305,
            Cipher::Aegis128L,
            Cipher::Aegis256,
        ] {
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
    fn aegis256_encrypt_decrypt() {
        let cipher = Cipher::Aegis256;
        let key = cipher.generate_key().unwrap();
        let plaintext = b"hello aegis-256".to_vec();
        let ciphertext = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
        let recovered = cipher.decrypt(key, ciphertext).unwrap();
        assert_eq!(plaintext, recovered);
    }

    const ALL_CIPHERS: [Cipher; 4] = [
        Cipher::Ascon128a,
        Cipher::ChaCha20Poly1305,
        Cipher::Aegis128L,
        Cipher::Aegis256,
    ];

    #[test]
    fn chunk_zero_matches_whole_payload_encryption() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"chunk zero identity".to_vec();
            assert_eq!(
                cipher.encrypt_chunk(&key, 0, plaintext.clone()).unwrap(),
                cipher.encrypt(key.clone(), plaintext).unwrap(),
                "{} chunk 0 must match legacy output",
                cipher.as_str()
            );
        }
    }

    #[test]
    fn chunk_roundtrip_all_ciphers() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            for index in [0u64, 1, 7, u64::MAX] {
                let plaintext = format!("chunk {index}").into_bytes();
                let ciphertext = cipher.encrypt_chunk(&key, index, plaintext.clone()).unwrap();
                let recovered = cipher.decrypt_chunk(&key, index, ciphertext).unwrap();
                assert_eq!(plaintext, recovered, "{} chunk {index}", cipher.as_str());
            }
        }
    }

    #[test]
    fn per_chunk_nonces_remove_keystream_reuse() {
        let cipher = Cipher::ChaCha20Poly1305;
        let key = cipher.generate_key().unwrap();
        let zeros = vec![0u8; 64];
        let ones = vec![0xFFu8; 64];

        // Under a shared nonce the keystream cancels: ct_a XOR ct_b == pt_a XOR pt_b.
        let leak_a = cipher.encrypt(key.clone(), zeros.clone()).unwrap();
        let leak_b = cipher.encrypt(key.clone(), ones.clone()).unwrap();
        assert!(leak_a[..64].iter().zip(&leak_b[..64]).all(|(a, b)| a ^ b == 0xFF));

        // Per-chunk nonces must break that relation.
        let ct_a = cipher.encrypt_chunk(&key, 1, zeros).unwrap();
        let ct_b = cipher.encrypt_chunk(&key, 2, ones).unwrap();
        assert!(!ct_a[..64].iter().zip(&ct_b[..64]).all(|(a, b)| a ^ b == 0xFF));
    }

    #[test]
    fn legacy_fixed_nonce_chunks_still_decrypt() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"pre-existing multi-chunk file".to_vec();
            let legacy = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
            assert_eq!(
                cipher.decrypt_chunk(&key, 3, legacy).unwrap(),
                plaintext,
                "{} legacy chunk must still decrypt",
                cipher.as_str()
            );
        }
    }

    #[test]
    fn chunk_at_wrong_index_fails_authentication() {
        let cipher = Cipher::Aegis128L;
        let key = cipher.generate_key().unwrap();
        let ciphertext = cipher.encrypt_chunk(&key, 2, b"reordered".to_vec()).unwrap();
        assert!(cipher.decrypt_chunk(&key, 3, ciphertext).is_err());
    }

    #[test]
    fn string_roundtrip_all_ciphers() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            for plaintext in ["file name.txt", "", "あいうえお 📁"] {
                let blob = cipher
                    .encrypt_string(key.clone(), plaintext.as_bytes().to_vec())
                    .unwrap();
                assert_eq!(
                    cipher.decrypt_string(key.clone(), blob).unwrap(),
                    plaintext.as_bytes(),
                    "{} roundtrip of {plaintext:?}",
                    cipher.as_str()
                );
            }
        }
    }

    #[test]
    fn string_encryptions_never_share_a_nonce() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"same name twice".to_vec();
            assert_ne!(
                cipher.encrypt_string(key.clone(), plaintext.clone()).unwrap(),
                cipher.encrypt_string(key, plaintext).unwrap(),
                "{} must randomize the nonce",
                cipher.as_str()
            );
        }
    }

    #[test]
    fn legacy_embedded_nonce_strings_still_decrypt() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"pre-existing metadata".to_vec();
            let legacy = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
            assert_eq!(
                cipher.decrypt_string(key, legacy).unwrap(),
                plaintext,
                "{} legacy string must still decrypt",
                cipher.as_str()
            );
        }
    }

    /// Cross-client anchor: the same vector is asserted in
    /// `web/tests/crypto-cipher.test.ts` and the Flutter client's
    /// `test/core/crypto/file_crypto_test.dart`. If any implementation of the
    /// nonce-prepend format drifts, one of the three fails. Encryption cannot
    /// be goldened because the nonce is random, so only decryption is pinned.
    #[test]
    fn string_golden_vector_decrypts() {
        let key = hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
            .unwrap();
        let blob = hex::decode(
            "a0a1a2a3a4a5a6a7a8a9aaabacadaeafbbe8a3087cc12efc536324b18fb194d014ab82478e8e43951d2d",
        )
        .unwrap();
        assert_eq!(
            Cipher::Ascon128a.decrypt_string(key, blob).unwrap(),
            b"hoodik.txt"
        );
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
