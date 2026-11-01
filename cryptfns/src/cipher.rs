use crate::{aegis, aegis256, aes, chacha};
use crate::error::{CryptoResult, Error};

/// Leading bytes of every chunk written by [`Cipher::encrypt_chunk`].
///
/// Chunks from earlier releases carry no header, so this marks the format.
/// Eight bytes keep the odds of a legacy chunk starting with it by chance at
/// 2^-64.  The last byte is the chunk format version.
pub const CHUNK_FORMAT_TAG: [u8; 8] = *b"HDKCHK\x00\x02";

/// Whether `chunk` is in the [`CHUNK_FORMAT_TAG`] format.
pub fn is_tagged_chunk(chunk: &[u8]) -> bool {
    chunk.starts_with(&CHUNK_FORMAT_TAG)
}

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

    /// How many bytes [`Cipher::encrypt_chunk`] adds to a chunk: the format
    /// tag, the per-chunk nonce and the AEAD tag.
    ///
    /// Measured by encrypting nothing rather than declared as a constant, so
    /// it cannot drift from what [`Cipher::encrypt_chunk`] actually produces.
    /// Callers that must state a ciphertext length before they have the
    /// ciphertext need this: a presigned upload URL is signed over the exact
    /// content length, and the bucket refuses a body of any other size.
    pub fn overhead(&self) -> CryptoResult<usize> {
        Ok(self
            .encrypt_chunk(&self.generate_key()?, 0, 1, Vec::new())?
            .len())
    }

    /// Encrypt one chunk of a `total_chunks`-chunk payload.
    ///
    /// Layout: [`CHUNK_FORMAT_TAG`] ‖ nonce ‖ ciphertext ‖ tag.  Every call
    /// draws a fresh random nonce, so no two chunk encryptions under one file
    /// key share a nonce, whichever save or position they belong to.  The
    /// associated data binds the format tag, the cipher id, the chunk index
    /// and the total chunk count, so a chunk only authenticates at the
    /// position, and in a payload of the length, it was written for.
    ///
    /// Nonce sizes bound how many chunks one key can safely encrypt.  The
    /// tightest is ChaCha20-Poly1305's 96-bit nonce: random draws collide
    /// with probability about n²/2⁹⁷, which stays below 2⁻³² up to 2³²
    /// chunks per key — 16 PiB at 4 MiB chunks, and keys here are
    /// per-file.  The 128-bit (Ascon-128a, AEGIS-128L) and 256-bit
    /// (AEGIS-256) nonces sit far below that.  Re-check this bound before
    /// wiring in a cipher with a shorter nonce.
    pub fn encrypt_chunk(
        &self,
        key: &[u8],
        chunk_index: u64,
        total_chunks: u64,
        plaintext: Vec<u8>,
    ) -> CryptoResult<Vec<u8>> {
        if chunk_index >= total_chunks {
            return Err(Error::InvalidChunk("chunk index out of range"));
        }

        let (key_len, nonce_len) = self.key_nonce_lengths();
        let mut nonce = vec![0u8; nonce_len];
        getrandom::getrandom(&mut nonce)?;

        let aad = self.chunk_aad(chunk_index, total_chunks);
        let ciphertext = self.encrypt_aad(self.blob(key, key_len, &nonce), plaintext, &aad)?;

        let mut out = Vec::with_capacity(CHUNK_FORMAT_TAG.len() + nonce_len + ciphertext.len());
        out.extend_from_slice(&CHUNK_FORMAT_TAG);
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    /// Decrypt one chunk of a `total_chunks`-chunk payload.
    ///
    /// The format is read from the chunk itself.  A chunk that starts with
    /// [`CHUNK_FORMAT_TAG`] is decoded strictly as written by
    /// [`Cipher::encrypt_chunk`]: there is no fallback, so it fails on any
    /// change to its bytes, index, total chunk count or cipher.  Anything else
    /// is a chunk written by an earlier release and goes through
    /// [`Cipher::decrypt_legacy_chunk`], so existing files stay readable.
    ///
    /// The associated data does not identify the save a chunk belongs to:
    /// two saves of the same file with the same chunk count produce chunks
    /// that are interchangeable at equal indices, and a complete earlier
    /// chunk set for a file remains decryptable as a whole.  Binding a
    /// per-save identifier would close that and needs the identifier
    /// threaded through every upload and download path.
    pub fn decrypt_chunk(
        &self,
        key: &[u8],
        chunk_index: u64,
        total_chunks: u64,
        ciphertext: Vec<u8>,
    ) -> CryptoResult<Vec<u8>> {
        if !is_tagged_chunk(&ciphertext) {
            // The legacy path predates total_chunks, so untagged chunks skip
            // the index-range check below; their AEAD tag still binds the
            // index through the nonce derivation.
            return self.decrypt_legacy_chunk(key, chunk_index, ciphertext);
        }

        if chunk_index >= total_chunks {
            return Err(Error::InvalidChunk("chunk index out of range"));
        }

        let (key_len, nonce_len) = self.key_nonce_lengths();
        let body = &ciphertext[CHUNK_FORMAT_TAG.len()..];
        if body.len() < nonce_len {
            return Err(Error::InvalidChunk("chunk too short"));
        }
        let (nonce, sealed) = body.split_at(nonce_len);

        let aad = self.chunk_aad(chunk_index, total_chunks);
        self.decrypt_aad(self.blob(key, key_len, nonce), sealed.to_vec(), &aad)
    }

    /// Decrypt a chunk written before [`CHUNK_FORMAT_TAG`] existed.
    ///
    /// Those releases derived the chunk nonce from the key blob by XOR-ing the
    /// little-endian index into it, and the earliest ones used the blob's
    /// nonce as-is for every chunk.  Both layouts are read here; nothing new
    /// is ever written in either.
    ///
    /// This path stays open for the life of a key: any chunk ever written
    /// for a file in a legacy layout keeps decrypting, even after later
    /// saves move the file to the tagged format.  Closing it would require
    /// re-encrypting existing data.
    pub fn decrypt_legacy_chunk(
        &self,
        key: &[u8],
        chunk_index: u64,
        ciphertext: Vec<u8>,
    ) -> CryptoResult<Vec<u8>> {
        if chunk_index == 0 {
            return self.decrypt(key.to_vec(), ciphertext);
        }
        self.decrypt(self.legacy_chunk_key(key, chunk_index), ciphertext.clone())
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

    /// Legacy chunk key blob for `chunk_index`: the little-endian index
    /// XOR-ed into the first 8 bytes of the embedded nonce.  Read-only path.
    fn legacy_chunk_key(&self, key: &[u8], chunk_index: u64) -> Vec<u8> {
        let (key_len, nonce_len) = self.key_nonce_lengths();
        let mut blob = key.to_vec();
        blob.resize(key_len + nonce_len, 0);
        for (i, b) in chunk_index.to_le_bytes().iter().enumerate() {
            blob[key_len + i] ^= b;
        }
        blob
    }

    /// The file key's key bytes followed by `nonce`.
    fn blob(&self, key: &[u8], key_len: usize, nonce: &[u8]) -> Vec<u8> {
        let mut blob = key.to_vec();
        blob.resize(key_len, 0);
        blob.extend_from_slice(nonce);
        blob
    }

    /// Associated data of a tagged chunk.
    fn chunk_aad(&self, chunk_index: u64, total_chunks: u64) -> Vec<u8> {
        let id = self.as_str().as_bytes();
        let mut aad = Vec::with_capacity(CHUNK_FORMAT_TAG.len() + 1 + id.len() + 16);
        aad.extend_from_slice(&CHUNK_FORMAT_TAG);
        aad.push(id.len() as u8);
        aad.extend_from_slice(id);
        aad.extend_from_slice(&chunk_index.to_le_bytes());
        aad.extend_from_slice(&total_chunks.to_le_bytes());
        aad
    }

    fn encrypt_aad(&self, key: Vec<u8>, plaintext: Vec<u8>, aad: &[u8]) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::encrypt_aad(key, plaintext, aad),
            Self::ChaCha20Poly1305 => chacha::encrypt_aad(key, plaintext, aad),
            Self::Aegis128L => aegis::encrypt_aad(key, plaintext, aad),
            Self::Aegis256 => aegis256::encrypt_aad(key, plaintext, aad),
        }
    }

    fn decrypt_aad(&self, key: Vec<u8>, ciphertext: Vec<u8>, aad: &[u8]) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Ascon128a => aes::decrypt_aad(key, ciphertext, aad),
            Self::ChaCha20Poly1305 => chacha::decrypt_aad(key, ciphertext, aad),
            Self::Aegis128L => aegis::decrypt_aad(key, ciphertext, aad),
            Self::Aegis256 => aegis256::decrypt_aad(key, ciphertext, aad),
        }
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
    fn chunk_roundtrip_all_ciphers() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            for (index, total) in [(0u64, 1u64), (1, 2), (7, 9), (u64::MAX - 1, u64::MAX)] {
                let plaintext = format!("chunk {index}").into_bytes();
                let ciphertext = cipher
                    .encrypt_chunk(&key, index, total, plaintext.clone())
                    .unwrap();
                assert!(is_tagged_chunk(&ciphertext));
                let recovered = cipher.decrypt_chunk(&key, index, total, ciphertext).unwrap();
                assert_eq!(plaintext, recovered, "{} chunk {index}", cipher.as_str());
            }
        }
    }

    #[test]
    fn empty_chunk_roundtrips() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let ciphertext = cipher.encrypt_chunk(&key, 0, 1, Vec::new()).unwrap();
            assert_eq!(ciphertext.len(), cipher.overhead().unwrap());
            assert!(cipher.decrypt_chunk(&key, 0, 1, ciphertext).unwrap().is_empty());
        }
    }

    #[test]
    fn overhead_matches_chunk_output() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let ciphertext = cipher.encrypt_chunk(&key, 3, 5, vec![7u8; 1000]).unwrap();
            assert_eq!(ciphertext.len(), 1000 + cipher.overhead().unwrap());
        }
    }

    #[test]
    fn same_key_and_index_never_share_a_keystream() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let zeros = vec![0u8; 64];
            let ones = vec![0xFFu8; 64];
            let header = CHUNK_FORMAT_TAG.len() + cipher.key_nonce_lengths().1;

            let ct_a = cipher.encrypt_chunk(&key, 0, 1, zeros.clone()).unwrap();
            let ct_b = cipher.encrypt_chunk(&key, 0, 1, ones).unwrap();
            assert_ne!(ct_a[..header], ct_b[..header], "{} headers must differ", cipher.as_str());
            assert!(
                !ct_a[header..header + 64]
                    .iter()
                    .zip(&ct_b[header..header + 64])
                    .all(|(a, b)| a ^ b == 0xFF),
                "{} bodies must be independent",
                cipher.as_str()
            );

            // Same plaintext twice: the ciphertexts still differ.
            let ct_c = cipher.encrypt_chunk(&key, 0, 1, zeros.clone()).unwrap();
            assert_ne!(ct_a, ct_c, "{}", cipher.as_str());
        }
    }

    #[test]
    fn tampered_chunk_fails() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let ciphertext = cipher.encrypt_chunk(&key, 1, 3, b"tamper me".to_vec()).unwrap();
            // Every byte after the format tag is covered: nonce, body and tag.
            for i in CHUNK_FORMAT_TAG.len()..ciphertext.len() {
                let mut bad = ciphertext.clone();
                bad[i] ^= 0x01;
                assert!(cipher.decrypt_chunk(&key, 1, 3, bad).is_err(), "{} byte {i}", cipher.as_str());
            }
        }
    }

    #[test]
    fn chunk_at_wrong_index_fails() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let ciphertext = cipher.encrypt_chunk(&key, 2, 4, b"reordered".to_vec()).unwrap();
            assert!(cipher.decrypt_chunk(&key, 3, 4, ciphertext.clone()).is_err());
            assert!(cipher.decrypt_chunk(&key, 0, 4, ciphertext).is_err());
        }
    }

    #[test]
    fn chunk_with_wrong_total_fails() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            // Truncating a 3-chunk payload to 2 chunks: chunk 1 now claims total 2.
            let ciphertext = cipher.encrypt_chunk(&key, 1, 3, b"middle".to_vec()).unwrap();
            assert!(cipher.decrypt_chunk(&key, 1, 2, ciphertext.clone()).is_err());
            // Extending it fails the same way.
            assert!(cipher.decrypt_chunk(&key, 1, 4, ciphertext).is_err());
        }
    }

    #[test]
    fn truncated_chunk_fails() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let ciphertext = cipher.encrypt_chunk(&key, 0, 1, b"cut short".to_vec()).unwrap();
            for len in [ciphertext.len() - 1, CHUNK_FORMAT_TAG.len() + 4, CHUNK_FORMAT_TAG.len()] {
                assert!(
                    cipher.decrypt_chunk(&key, 0, 1, ciphertext[..len].to_vec()).is_err(),
                    "{} len {len}",
                    cipher.as_str()
                );
            }
        }
    }

    #[test]
    fn chunk_spliced_from_another_payload_fails() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            // Chunk 1 of a 2-chunk save dropped into a 3-chunk save.
            let other = cipher.encrypt_chunk(&key, 1, 2, b"other save".to_vec()).unwrap();
            assert!(cipher.decrypt_chunk(&key, 1, 3, other).is_err());
        }
    }

    #[test]
    fn chunk_under_another_cipher_fails() {
        // AEGIS-128L and Ascon-128a share key and nonce lengths, so only the
        // cipher id in the associated data separates them.
        let key = Cipher::Aegis128L.generate_key().unwrap();
        let ciphertext = Cipher::Aegis128L.encrypt_chunk(&key, 0, 1, b"x".to_vec()).unwrap();
        assert!(Cipher::Ascon128a.decrypt_chunk(&key, 0, 1, ciphertext).is_err());
    }

    #[test]
    fn tagged_chunk_has_no_legacy_fallback() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"legacy body".to_vec();
            // A legacy ciphertext dressed with the format tag must not be
            // accepted by falling through to the legacy decoder.
            for legacy in [
                cipher.encrypt(key.clone(), plaintext.clone()).unwrap(),
                cipher.encrypt(cipher.legacy_chunk_key(&key, 2), plaintext.clone()).unwrap(),
            ] {
                let mut dressed = CHUNK_FORMAT_TAG.to_vec();
                dressed.extend_from_slice(&legacy);
                assert!(cipher.decrypt_chunk(&key, 2, 3, dressed).is_err(), "{}", cipher.as_str());
            }
        }
    }

    #[test]
    fn index_past_total_is_rejected() {
        let cipher = Cipher::Aegis128L;
        let key = cipher.generate_key().unwrap();
        assert!(cipher.encrypt_chunk(&key, 1, 1, b"x".to_vec()).is_err());
        let ciphertext = cipher.encrypt_chunk(&key, 0, 1, b"x".to_vec()).unwrap();
        assert!(cipher.decrypt_chunk(&key, 1, 1, ciphertext).is_err());
    }

    #[test]
    fn legacy_index_derived_chunks_still_decrypt() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"chunk from a 2.x release".to_vec();
            for index in [0u64, 1, 5] {
                let legacy = cipher
                    .encrypt(cipher.legacy_chunk_key(&key, index), plaintext.clone())
                    .unwrap();
                assert_eq!(
                    cipher.decrypt_chunk(&key, index, 6, legacy).unwrap(),
                    plaintext,
                    "{} legacy chunk {index}",
                    cipher.as_str()
                );
            }
        }
    }

    #[test]
    fn legacy_fixed_nonce_chunks_still_decrypt() {
        for cipher in ALL_CIPHERS {
            let key = cipher.generate_key().unwrap();
            let plaintext = b"pre-existing multi-chunk file".to_vec();
            let legacy = cipher.encrypt(key.clone(), plaintext.clone()).unwrap();
            assert_eq!(
                cipher.decrypt_chunk(&key, 3, 4, legacy).unwrap(),
                plaintext,
                "{} legacy chunk must still decrypt",
                cipher.as_str()
            );
        }
    }

    #[test]
    fn legacy_chunk_at_wrong_index_fails() {
        let cipher = Cipher::Aegis128L;
        let key = cipher.generate_key().unwrap();
        let legacy = cipher.encrypt(cipher.legacy_chunk_key(&key, 2), b"x".to_vec()).unwrap();
        assert!(cipher.decrypt_chunk(&key, 3, 4, legacy).is_err());
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

    /// Cross-client anchor for the tagged chunk layout: the same vector is
    /// asserted in `web/tests/crypto-cipher.test.ts` and the Flutter client's
    /// `test/core/crypto/file_crypto_test.dart`. If any implementation of the
    /// tag ‖ nonce ‖ ciphertext layout or the associated data drifts, one of
    /// the three fails. Encryption cannot be goldened because the nonce is
    /// random, so only decryption is pinned. AEGIS-128L chunk 1 of 3.
    #[test]
    fn chunk_golden_vector_decrypts() {
        let key = hex::decode("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f")
            .unwrap();
        let chunk = hex::decode(
            "48444b43484b0002a0a1a2a3a4a5a6a7a8a9aaabacadaeaf555f6e84c13c89dd00b3fe3a08937b3a8568eb1fbdc286e675a515fb",
        )
        .unwrap();
        assert_eq!(
            Cipher::Aegis128L.decrypt_chunk(&key, 1, 3, chunk).unwrap(),
            b"hoodik chunk"
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
