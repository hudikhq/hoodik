//! Re-index one file against the keyed search scheme.
//!
//! The migration that introduced keyed tags dropped every old index row,
//! because those rows were reversible and keeping them through a transition
//! would have kept the readable copy alive indefinitely. Nothing server-side
//! can rebuild them: the tags are keyed on material only the client holds. So
//! each client walks its own files once and re-indexes them through here.

use ::error::AppResult;
use entity::file_tokens::{DigestTags, SearchTags};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Reindex {
    /// Re-keyed `name_hash`. Migrated rows still carry the old unsalted
    /// digest of the plaintext name, which is the same leak in a second
    /// place, so the sweep replaces it alongside the tags.
    pub name_hash: Option<String>,
    /// The account fingerprint the tags were keyed under. The root search
    /// key is derived from the private key, so a session that started the
    /// sweep before a key-rotation ceremony committed would otherwise write
    /// a 32-hex `name_hash` that looks "done" and never matches a query
    /// under the new key. Compared to `users.fingerprint` at write time;
    /// a mismatch leaves the file pending.
    pub fingerprint: Option<String>,
    pub search_tokens_root: Option<Vec<String>>,
    pub search_tokens_file: Option<Vec<String>>,
    /// Note-body tokens, written to `source=content`. Absent on regular files
    /// and on clients that still mix name and body into `search_tokens_*`.
    pub content_tokens_root: Option<Vec<String>>,
    pub content_tokens_file: Option<Vec<String>>,
    /// Content digests re-keyed under the file's search key, replacing the
    /// bare digests migrated rows still carry — the third copy of the same
    /// leak. The sweep computes them from the stored values, so nothing has
    /// to be re-downloaded, and a client that can hold the file key can
    /// still run its resume equality check against the keyed form.
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
    /// The digest tags that make the re-keyed values findable by pasting a
    /// digest into search. Separate from the word tokens because they land in
    /// the digest scopes, which renames never touch.
    pub digest_tokens_root: Option<Vec<String>>,
    pub digest_tokens_file: Option<Vec<String>>,
}

/// The keyed digest columns a re-index rewrites, `None` meaning "leave it".
pub struct KeyedHashes {
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
}

impl Validation for Reindex {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(name_hash), rule_required!(fingerprint)]
    }
}

impl Reindex {
    pub fn into_parts(
        self,
    ) -> AppResult<(
        String,
        String,
        SearchTags,
        SearchTags,
        DigestTags,
        KeyedHashes,
    )> {
        let data = self.validate()?;
        let name_hash = data.name_hash.unwrap();
        let fingerprint = data.fingerprint.unwrap();

        // This route exists to replace the reversible digest, so of all
        // places it must not accept one back. Same refusal as create and
        // rename: a keyed hash is half the length, so the shapes never
        // collide.
        if cryptfns::search::is_legacy_name_hash(&name_hash) {
            return Err(::error::Error::UpgradeRequired(
                "client_too_old_for_search".to_string(),
            ));
        }

        // The same refusal for the digest columns themselves: a sweep that
        // would write a bare digest back defeats its own purpose. MD5 shares
        // the tag's shape and cannot be told apart here.
        for value in [&data.md5, &data.sha1, &data.sha256, &data.blake2b]
            .into_iter()
            .flatten()
        {
            if cryptfns::search::is_bare_digest(value) {
                return Err(::error::Error::UpgradeRequired(
                    "client_too_old_for_search".to_string(),
                ));
            }
        }

        Ok((
            name_hash,
            fingerprint,
            SearchTags::new(data.search_tokens_root, data.search_tokens_file),
            SearchTags::new(data.content_tokens_root, data.content_tokens_file),
            DigestTags::new(data.digest_tokens_root, data.digest_tokens_file),
            KeyedHashes {
                md5: data.md5,
                sha1: data.sha1,
                sha256: data.sha256,
                blake2b: data.blake2b,
            },
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn name_hash_is_required() {
        assert!(Reindex::default().into_parts().is_err());
    }

    #[test]
    fn fingerprint_is_required() {
        let result = Reindex {
            name_hash: Some("a".repeat(32)),
            ..Default::default()
        }
        .into_parts();

        assert!(result.is_err());
    }

    #[test]
    fn a_legacy_digest_is_refused() {
        let result = Reindex {
            name_hash: Some(cryptfns::sha256::digest("secret name".as_bytes())),
            fingerprint: Some("fp".to_string()),
            ..Default::default()
        }
        .into_parts();

        assert!(
            matches!(result, Err(::error::Error::UpgradeRequired(_))),
            "the route built to replace the reversible digest accepted one back"
        );
    }

    #[test]
    fn a_bare_digest_column_is_refused() {
        let result = Reindex {
            name_hash: Some("a".repeat(32)),
            fingerprint: Some("fp".to_string()),
            sha1: Some(cryptfns::sha256::digest("bytes".as_bytes())),
            ..Default::default()
        }
        .into_parts();

        assert!(
            matches!(result, Err(::error::Error::UpgradeRequired(_))),
            "a digest column accepted a value that is not a keyed tag"
        );
    }

    #[test]
    fn tags_and_keyed_hashes_ride_along_with_the_name_hash() {
        let (name_hash, fingerprint, tags, content, digests, hashes) = Reindex {
            name_hash: Some("abc".to_string()),
            fingerprint: Some("fp".to_string()),
            search_tokens_root: Some(vec!["a3f1:2".to_string()]),
            search_tokens_file: Some(vec!["9c22:1".to_string()]),
            sha256: Some("b".repeat(32)),
            digest_tokens_root: Some(vec![format!("{}:1", "c".repeat(32))]),
            ..Default::default()
        }
        .into_parts()
        .unwrap();

        assert_eq!(name_hash, "abc");
        assert_eq!(fingerprint, "fp");
        assert_eq!(tags.root.unwrap().len(), 1);
        assert_eq!(tags.file.unwrap().len(), 1);
        assert!(content.root.is_none());
        assert!(content.file.is_none());
        assert_eq!(digests.root.unwrap().len(), 1);
        assert!(digests.file.is_none());
        assert_eq!(hashes.sha256.unwrap(), "b".repeat(32));
        assert!(hashes.md5.is_none());
    }

    #[test]
    fn content_tags_ride_along_separately_from_the_name() {
        let (_, _, tags, content, _, _) = Reindex {
            name_hash: Some("abc".to_string()),
            fingerprint: Some("fp".to_string()),
            search_tokens_root: Some(vec!["a3f1:2".to_string()]),
            content_tokens_root: Some(vec!["b4e2:1".to_string()]),
            content_tokens_file: Some(vec!["c5d3:1".to_string()]),
            ..Default::default()
        }
        .into_parts()
        .unwrap();

        assert_eq!(tags.root.unwrap().len(), 1);
        assert!(tags.file.is_none());
        assert_eq!(content.root.unwrap().len(), 1);
        assert_eq!(content.file.unwrap().len(), 1);
    }
}
