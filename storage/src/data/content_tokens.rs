//! Note-body search tags, written on their own.
//!
//! Same keyed `"{tag}:{weight}"` wire form as every other index write; the
//! route replaces `source=content` and nothing else.
//!
//! It exists for restore. A restore names a version and carries no body, and
//! the server holds only ciphertext, so nothing on that request can produce
//! the tags for the text being restored. The route clears them and enrols the
//! file in the owner's sweep instead — correct, but it leaves the note
//! unfindable by its own words until the sweep reaches it. A client that has
//! just decrypted the restored version already holds what is needed, and this
//! is where it puts it.

use ::error::AppResult;
use entity::file_tokens::SearchTags;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContentTokens {
    pub content_tokens_root: Option<Vec<String>>,
    pub content_tokens_file: Option<Vec<String>>,
}

impl ContentTokens {
    /// No cap, unlike the extra source: a note's body is as long as the note
    /// is, and one truncated at some limit would be worse than none — it
    /// would answer some searches and silently miss others.
    pub fn into_search_tags(self) -> AppResult<SearchTags> {
        Ok(SearchTags::new(
            self.content_tokens_root,
            self.content_tokens_file,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn both_scopes_survive_the_conversion() {
        let tags = ContentTokens {
            content_tokens_root: Some(vec!["a3f1:1".to_string()]),
            content_tokens_file: Some(vec!["9c22:1".to_string()]),
        }
        .into_search_tags()
        .unwrap();

        assert_eq!(tags.root.unwrap(), vec!["a3f1:1".to_string()]);
        assert_eq!(tags.file.unwrap(), vec!["9c22:1".to_string()]);
    }

    #[test]
    fn an_absent_scope_stays_absent() {
        // `replace_source` leaves a scope it was given nothing for alone, so
        // an editor who cannot produce the owner's root tags does not wipe
        // them by sending only their own.
        let tags = ContentTokens {
            content_tokens_root: None,
            content_tokens_file: Some(vec!["9c22:1".to_string()]),
        }
        .into_search_tags()
        .unwrap();

        assert!(tags.root.is_none());
        assert!(tags.file.is_some());
    }
}
