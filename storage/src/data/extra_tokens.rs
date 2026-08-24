//! Extra search tags for a file.
//!
//! A later client can attach hashed context (OCR, image labels, and so on)
//! without another schema change. The tags are the same keyed `"{tag}:{weight}"`
//! entries every other index write uses — not a second hasher, and never
//! plaintext. The route replaces `source=extra` only.

use ::error::{AppResult, Error};
use entity::file_tokens::SearchTags;
use serde::{Deserialize, Serialize};

/// Hard cap on extra tags per scope. OCR and similar producers should stay
/// well under this; it exists so a buggy client cannot inflate the index
/// without bound.
pub const MAX_EXTRA_TOKENS: usize = 128;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ExtraTokens {
    pub search_tokens_root: Option<Vec<String>>,
    pub search_tokens_file: Option<Vec<String>>,
    /// Accepted as the root list when `search_tokens_root` is absent. Same
    /// keyed `"{tag}:{weight}"` entries as create — not the retired SHA-256
    /// digest form, which cannot match this index.
    pub search_tokens_hashed: Option<Vec<String>>,
}

impl ExtraTokens {
    /// Reject a list longer than [`MAX_EXTRA_TOKENS`] before any delete.
    ///
    /// Count is the raw entries the client sent, not the ones `from_wire`
    /// keeps: 129 junk strings is still a 129-entry payload.
    pub fn into_search_tags(self) -> AppResult<SearchTags> {
        let root = self.search_tokens_root.or(self.search_tokens_hashed);
        for list in [&root, &self.search_tokens_file] {
            if list.as_ref().is_some_and(|v| v.len() > MAX_EXTRA_TOKENS) {
                return Err(Error::BadRequest("extra_tokens_limit".to_string()));
            }
        }

        Ok(SearchTags::new(root, self.search_tokens_file))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn one_hundred_and_twenty_eight_is_accepted() {
        let tags = ExtraTokens {
            search_tokens_root: Some(vec!["a3f1:1".to_string(); MAX_EXTRA_TOKENS]),
            search_tokens_file: Some(vec!["9c22:1".to_string(); MAX_EXTRA_TOKENS]),
            ..Default::default()
        }
        .into_search_tags()
        .unwrap();

        assert_eq!(tags.root.unwrap().len(), MAX_EXTRA_TOKENS);
        assert_eq!(tags.file.unwrap().len(), MAX_EXTRA_TOKENS);
    }

    #[test]
    fn one_hundred_and_twenty_nine_is_rejected() {
        let err = ExtraTokens {
            search_tokens_root: Some(vec!["a3f1:1".to_string(); MAX_EXTRA_TOKENS + 1]),
            ..Default::default()
        }
        .into_search_tags()
        .unwrap_err();

        assert!(matches!(err, Error::BadRequest(ref m) if m == "extra_tokens_limit"));
    }

    #[test]
    fn hashed_is_the_root_list_when_root_is_absent() {
        let tags = ExtraTokens {
            search_tokens_hashed: Some(vec!["a3f1:1".to_string()]),
            ..Default::default()
        }
        .into_search_tags()
        .unwrap();

        assert_eq!(tags.root.unwrap(), vec!["a3f1:1".to_string()]);
        assert!(tags.file.is_none());
    }
}
