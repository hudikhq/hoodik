//! Keyed tags for the search index.
//!
//! The index stores HMAC tags rather than bare token digests, so read access
//! to the database no longer reverses a file name or a note body back into
//! words. Two keys, both derived on the client and neither ever sent to the
//! server:
//!
//! - The **root key** comes from the account's private key. It tags everything
//!   the user owns, which keeps their own search to one tag per query word.
//! - The **file key** comes from the file's own encryption key. It tags the
//!   same tokens a second time, and that key already reaches every share
//!   recipient inside `user_files.encrypted_key` — so a share grant needs no
//!   re-index, at any scale.
//!
//! A client therefore never sends file tags for files it owns, the two tag
//! sets can never match the same row, and weight-based ranking needs no
//! deduplication.

use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};

use crate::error::{CryptoResult, Error};

const ROOT_INFO: &[u8] = b"hoodik-search-root-v1";
const FILE_INFO: &[u8] = b"hoodik-search-file-v1";

pub const KEY_LENGTH: usize = 32;

/// Tag width in bytes before hex encoding. 128 bits leaves collisions far
/// below the rate at which the client would notice one, and a note body
/// indexes every distinct word it contains, so the column width is worth
/// spending care on.
pub const TAG_LENGTH: usize = 16;

type HmacSha256 = Hmac<Sha256>;

/// Derive the account-wide search key from a private key PEM.
///
/// The input is the account's wrapping private key on a curve25519 account and
/// its RSA private key on a legacy one. Either way the DER body is the key
/// material, so the derivation does not depend on PEM line wrapping or label
/// text, and it changes only when the key itself does. An account migrating
/// from RSA to curve25519 therefore re-indexes, which rides along with the
/// rewrap sweep that migration already performs.
pub fn root_key(private_key: &str) -> CryptoResult<[u8; KEY_LENGTH]> {
    expand(&pem_body(private_key)?, ROOT_INFO)
}

/// Derive a file's search key from the symmetric key the file is encrypted
/// with. Every recipient of a share can compute this the moment they can open
/// the file, which is what makes sharing free of index work.
pub fn file_key(file_key: &[u8]) -> CryptoResult<[u8; KEY_LENGTH]> {
    if file_key.is_empty() {
        return Err(Error::InvalidLength("search file key input is empty"));
    }

    expand(file_key, FILE_INFO)
}

/// Whether `name_hash` is a pre-keyed SHA-256 digest (64 chars) rather than a
/// keyed tag (`TAG_LENGTH` bytes = 32 hex chars).
///
/// Clients from before keyed search send `sha256(name)` here — the reversible
/// digest the keyed scheme exists to remove. A write carrying one is refused
/// rather than stored, so an old client cannot re-introduce the leak the
/// migration just purged. Length alone decides, matching the pending-reindex
/// query that derives "still legacy" from `LENGTH(name_hash) = 64`: accepting
/// a 64-char value that query counts as legacy would store a row that reports
/// itself pending forever.
pub fn is_legacy_name_hash(name_hash: &str) -> bool {
    name_hash.len() == 64
}

/// Whether `value` has the shape of a bare content digest: 40, 64 or 128 hex
/// chars is SHA-1, SHA-256 or BLAKE2b, none of which a keyed tag
/// (`TAG_LENGTH` bytes = 32 hex chars) can be. The digest columns store keyed
/// tags, so writes refuse these shapes the way the name paths refuse the
/// legacy name digest. Bare MD5 shares the tag's 32-hex shape and cannot be
/// told apart here; the client sweeps' sibling rule covers it.
pub fn is_bare_digest(value: &str) -> bool {
    matches!(value.len(), 40 | 64 | 128) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Tag one value under `key`. Used for single strings such as a file's
/// `name_hash`; token lists go through [`tag_tokens`].
pub fn tag(key: &[u8], value: &str) -> CryptoResult<String> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| Error::KeyEncoding(format!("search tag key rejected: {e}")))?;
    mac.update(value.as_bytes());

    Ok(hex::encode(&mac.finalize().into_bytes()[..TAG_LENGTH]))
}

/// Parse the `"{tag}:{weight}"` entries a client sends when indexing a file.
///
/// Malformed entries are dropped rather than failing the write: a tag the
/// server cannot parse is one the client will never match against either, and
/// refusing the whole upload over it would cost the file, not just its
/// searchability.
pub fn from_wire(entries: Vec<String>) -> Vec<(String, i32)> {
    entries
        .into_iter()
        .filter_map(|entry| {
            let (tag, weight) = entry.split_once(':')?;
            if tag.is_empty() {
                return None;
            }

            Some((tag.to_string(), weight.parse::<i32>().ok()?))
        })
        .collect()
}

fn expand(ikm: &[u8], info: &[u8]) -> CryptoResult<[u8; KEY_LENGTH]> {
    let mut key = [0u8; KEY_LENGTH];
    Hkdf::<Sha512>::new(None, ikm)
        .expand(info, &mut key)
        .map_err(|e| Error::KeyEncoding(e.to_string()))?;

    Ok(key)
}

/// Decode a PEM container down to its DER body. Deliberately label-agnostic:
/// the caller has already established which key it holds, and every key type
/// this crate deals in reduces to one base64 body between the armour lines.
fn pem_body(pem: &str) -> CryptoResult<Vec<u8>> {
    let body = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>()
        .replace([' ', '\t', '\r'], "");

    if body.is_empty() {
        return Err(Error::KeyEncoding(
            "private key has no PEM body".to_string(),
        ));
    }

    crate::base64::decode(&body).map_err(|e| Error::KeyEncoding(e.to_string()))
}

#[cfg(feature = "tokenizer")]
mod tokens {
    use super::*;
    use crate::tokenizer::Token;

    /// Tokenize `input` and tag every distinct token under `key`, preserving
    /// the per-token weight the ranking depends on.
    ///
    /// Case is folded here, in the one place both indexing and querying pass
    /// through, so the two can never disagree. The tokenizer is cased, so
    /// without this a note body indexed as written would never match a query
    /// the search box had lowercased — capitalized words silently unfindable.
    pub fn tag_tokens(key: &[u8], input: &str) -> CryptoResult<Vec<Token>> {
        let mut tokens = crate::tokenizer::into_tokens(&input.to_lowercase())?;

        for token in tokens.iter_mut() {
            token.token = tag(key, &token.token)?;
        }

        Ok(tokens)
    }
}

#[cfg(feature = "tokenizer")]
pub use tokens::tag_tokens;

#[cfg(test)]
mod test {
    use super::*;

    /// A throwaway wrapping key, generated once and pinned here so the
    /// derivation stays reproducible across runs.
    fn private_key() -> String {
        crate::ecdh::private::generate().unwrap()
    }

    #[test]
    fn root_key_is_stable_and_domain_separated() {
        let pem = private_key();

        assert_eq!(root_key(&pem).unwrap(), root_key(&pem).unwrap());

        let body = pem_body(&pem).unwrap();
        assert_ne!(root_key(&pem).unwrap(), file_key(&body).unwrap());
    }

    #[test]
    fn root_key_survives_pem_reformatting() {
        let pem = private_key();
        let rewrapped = pem.replace('\n', "\r\n");

        assert_eq!(root_key(&pem).unwrap(), root_key(&rewrapped).unwrap());
    }

    #[test]
    fn different_keys_tag_the_same_word_differently() {
        let a = root_key(&private_key()).unwrap();
        let b = root_key(&private_key()).unwrap();

        assert_ne!(tag(&a, "invoice").unwrap(), tag(&b, "invoice").unwrap());
    }

    #[test]
    fn tag_is_deterministic_and_the_expected_width() {
        let key = root_key(&private_key()).unwrap();

        assert_eq!(tag(&key, "invoice").unwrap(), tag(&key, "invoice").unwrap());
        assert_ne!(
            tag(&key, "invoice").unwrap(),
            tag(&key, "invoices").unwrap()
        );
        assert_eq!(tag(&key, "invoice").unwrap().len(), TAG_LENGTH * 2);
    }

    /// The whole point of the change: a tag must not be the token's bare
    /// digest, which is what a rainbow table over the BERT vocabulary needs.
    #[test]
    fn tag_is_not_the_bare_digest() {
        let key = root_key(&private_key()).unwrap();

        assert_ne!(
            tag(&key, "Hello").unwrap(),
            sha256::digest("Hello".as_bytes())
        );
    }

    #[test]
    fn wire_entries_parse_and_drop_junk() {
        let parsed = from_wire(vec![
            "a3f1:2".to_string(),
            "9c22:1".to_string(),
            "missing-weight".to_string(),
            ":3".to_string(),
            "bad:weight".to_string(),
        ]);

        assert_eq!(
            parsed,
            vec![("a3f1".to_string(), 2), ("9c22".to_string(), 1)]
        );
    }

    #[test]
    fn empty_file_key_is_rejected() {
        assert!(file_key(&[]).is_err());
    }

    #[test]
    fn legacy_name_hash_is_any_64_char_value() {
        // sha256 hex, the pre-keyed shape.
        assert!(is_legacy_name_hash(&"a".repeat(64)));
        // A keyed tag is half as long.
        assert!(!is_legacy_name_hash(&"a".repeat(TAG_LENGTH * 2)));
        // Not hex, but the pending query counts length alone — storing this
        // would leave the row pending forever, so it is refused all the same.
        assert!(is_legacy_name_hash(&"g".repeat(64)));
        assert!(!is_legacy_name_hash(""));
    }

    #[test]
    fn bare_digest_shapes_are_the_three_reversible_lengths() {
        // SHA-1, SHA-256, BLAKE2b.
        assert!(is_bare_digest(&"a".repeat(40)));
        assert!(is_bare_digest(&"a".repeat(64)));
        assert!(is_bare_digest(&"a".repeat(128)));
        // A keyed tag, and things that are no digest at all.
        assert!(!is_bare_digest(&"a".repeat(TAG_LENGTH * 2)));
        assert!(!is_bare_digest(&"g".repeat(64)));
        assert!(!is_bare_digest(""));
    }

    /// A query the search box lowercased has to match an index built from the
    /// note as written. The fold lives in `tag_tokens`, so both sides agree.
    #[cfg(feature = "tokenizer")]
    #[test]
    fn tag_tokens_folds_case() {
        let key = root_key(&private_key()).unwrap();

        let cased = crate::tokenizer::into_string(tag_tokens(&key, "Berlin Meetup").unwrap());
        let lower = crate::tokenizer::into_string(tag_tokens(&key, "berlin meetup").unwrap());

        assert_eq!(cased, lower);
    }

    /// A pinned vector: this exact key and input must produce this exact tag
    /// string in every client. The app's and web's suites assert the same
    /// value, so a divergence in tokenization, case-folding or the HMAC — a
    /// WASM/FFI/server skew — fails a test rather than silently splitting an
    /// account's index in two. If the tokenizer or tag scheme is ever changed
    /// on purpose, regenerate this in all three suites together.
    #[cfg(feature = "tokenizer")]
    #[test]
    fn golden_cross_client_tag_vector() {
        let key: Vec<u8> = (0u8..32).collect();
        let encoded = crate::tokenizer::into_string(tag_tokens(&key, "Invoice Q1").unwrap());

        assert_eq!(
            encoded,
            "ade2702652df2b527ea85d06ea18cc2a:1;\
             81a20aa0d8d8b149b992f4d641fffcad:1;\
             e9e098de1b057acdc1f7eafdd37a96a5:1;\
             e48f3669b623c473d2ae2e75739fd62f:1;\
             6520fd80f2b3010402038bcc9af77100:1"
        );
    }

    #[test]
    fn malformed_pem_is_rejected() {
        assert!(root_key("-----BEGIN NOTHING-----\n-----END NOTHING-----").is_err());
    }

    /// Every token the tokenizer produces must come back tagged, carrying the
    /// weight ranking depends on. Asserted against the tokenizer's own output
    /// rather than against a guessed word: the vocabulary is wordpiece and
    /// cased, so a term like "invoice" may well arrive as several tokens.
    #[cfg(feature = "tokenizer")]
    #[test]
    fn tagged_tokens_keep_their_weights() {
        let key = root_key(&private_key()).unwrap();
        let input = "invoice invoice draft";

        let raw = crate::tokenizer::into_tokens(input).unwrap();
        let tagged = tag_tokens(&key, input).unwrap();

        assert_eq!(raw.len(), tagged.len());

        for token in &raw {
            let expected = tag(&key, &token.token).unwrap();
            let found = tagged
                .iter()
                .find(|t| t.token == expected)
                .expect("every token is tagged");

            assert_eq!(found.weight, token.weight);
        }

        // The repeated term has to show up as a weight somewhere, however the
        // tokenizer chose to split it.
        assert!(tagged.iter().any(|t| t.weight >= 2));
        assert!(tagged.iter().all(|t| t.token.len() == TAG_LENGTH * 2));
    }
}
