//! Re-index one file against the keyed search scheme.
//!
//! The migration that introduced keyed tags dropped every old index row,
//! because those rows were reversible and keeping them through a transition
//! would have kept the readable copy alive indefinitely. Nothing server-side
//! can rebuild them: the tags are keyed on material only the client holds. So
//! each client walks its own files once and re-indexes them through here.

use ::error::AppResult;
use entity::file_tokens::SearchTags;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Reindex {
    /// Re-keyed `name_hash`. Migrated rows still carry the old unsalted
    /// digest of the plaintext name, which is the same leak in a second
    /// place, so the sweep replaces it alongside the tags.
    pub name_hash: Option<String>,
    pub search_tokens_root: Option<Vec<String>>,
    pub search_tokens_file: Option<Vec<String>>,
}

impl Validation for Reindex {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(name_hash)]
    }
}

impl Reindex {
    pub fn into_parts(self) -> AppResult<(String, SearchTags)> {
        let data = self.validate()?;
        let name_hash = data.name_hash.unwrap();

        // This route exists to replace the reversible digest, so of all
        // places it must not accept one back. Same refusal as create and
        // rename: a keyed hash is half the length, so the shapes never
        // collide.
        if cryptfns::search::is_legacy_name_hash(&name_hash) {
            return Err(::error::Error::UpgradeRequired(
                "client_too_old_for_search".to_string(),
            ));
        }

        Ok((
            name_hash,
            SearchTags::new(data.search_tokens_root, data.search_tokens_file),
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
    fn a_legacy_digest_is_refused() {
        let result = Reindex {
            name_hash: Some(cryptfns::sha256::digest("secret name".as_bytes())),
            search_tokens_root: None,
            search_tokens_file: None,
        }
        .into_parts();

        assert!(
            matches!(result, Err(::error::Error::UpgradeRequired(_))),
            "the route built to replace the reversible digest accepted one back"
        );
    }

    #[test]
    fn tags_ride_along_with_the_name_hash() {
        let (name_hash, tags) = Reindex {
            name_hash: Some("abc".to_string()),
            search_tokens_root: Some(vec!["a3f1:2".to_string()]),
            search_tokens_file: Some(vec!["9c22:1".to_string()]),
        }
        .into_parts()
        .unwrap();

        assert_eq!(name_hash, "abc");
        assert_eq!(tags.root.unwrap().len(), 1);
        assert_eq!(tags.file.unwrap().len(), 1);
    }
}
