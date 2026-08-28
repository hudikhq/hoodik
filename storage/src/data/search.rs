use ::error::{AppResult, Error};
use entity::{option_string_to_uuid, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Search {
    pub dir_id: Option<String>,
    /// Query words tagged under the caller's account-wide search key. Matches
    /// everything they own, one tag per word regardless of how large the drive
    /// is.
    pub root_tags: Option<Vec<String>>,
    /// The same words tagged once per file that is shared *with* the caller,
    /// under each file's own key. Callers never send these for files they own,
    /// so a file can only ever match through one scope and the weight ranking
    /// stays honest.
    pub file_tags: Option<Vec<String>>,
    /// The whole query hashed as a filename under the caller's key. A file
    /// whose `name_hash` equals it is the file the user typed the name of —
    /// ranked above every token match, so pasting a filename always surfaces
    /// that file first instead of whichever document happens to share the
    /// most words with it.
    pub name_hash: Option<String>,
    pub limit: Option<u64>,
    pub skip: Option<u64>,
    pub editable: Option<bool>,
    /// Withhold `encrypted_thumbnail` from the results and report only
    /// `has_thumbnail`. Absent means full rows — the compatible default
    /// for older clients.
    pub compact: Option<bool>,
    /// Plaintext query from clients that predate client-side tokenization.
    /// Refused outright: see [`Search::reject_legacy`].
    pub search: Option<String>,
    /// Bare SHA-256 token digests from clients that predate keyed tags.
    /// Refused outright: see [`Search::reject_legacy`].
    pub search_tokens_hashed: Option<Vec<String>>,
}

impl Validation for Search {}

pub type SearchData = (
    Option<Uuid>,
    Vec<String>,
    Vec<String>,
    Option<String>,
    Option<u64>,
    Option<u64>,
    Option<bool>,
);

impl Search {
    /// Refuse a query built by a client that predates keyed tags.
    ///
    /// Those clients send either a plaintext query or bare SHA-256 digests of
    /// the query words. Neither can be served: the index no longer holds
    /// anything they would match, and honouring them would mean writing
    /// reversible material back into the database. Answering with an empty
    /// result set would be worse than an error — a user whose search silently
    /// returns nothing concludes their files are gone.
    pub fn reject_legacy(&self) -> AppResult<()> {
        if self.search.is_some() || self.search_tokens_hashed.is_some() {
            return Err(Error::UpgradeRequired(
                "client_too_old_for_search".to_string(),
            ));
        }

        Ok(())
    }

    pub fn into_tuple(self) -> SearchData {
        (
            option_string_to_uuid(self.dir_id),
            sanitize(self.root_tags),
            sanitize(self.file_tags),
            self.name_hash.filter(|hash| !hash.is_empty()),
            self.limit,
            self.skip,
            self.editable,
        )
    }
}

fn sanitize(tags: Option<Vec<String>>) -> Vec<String> {
    tags.unwrap_or_default()
        .into_iter()
        .filter(|tag| !tag.is_empty())
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn legacy_plaintext_query_is_refused() {
        let search = Search {
            search: Some("invoice".to_string()),
            ..Default::default()
        };

        assert!(matches!(
            search.reject_legacy(),
            Err(Error::UpgradeRequired(_))
        ));
    }

    #[test]
    fn legacy_digest_query_is_refused() {
        let search = Search {
            search_tokens_hashed: Some(vec!["deadbeef:1".to_string()]),
            ..Default::default()
        };

        assert!(matches!(
            search.reject_legacy(),
            Err(Error::UpgradeRequired(_))
        ));
    }

    #[test]
    fn tagged_query_is_accepted() {
        let search = Search {
            root_tags: Some(vec!["a3f1".to_string()]),
            ..Default::default()
        };

        assert!(search.reject_legacy().is_ok());
    }

    #[test]
    fn empty_tags_are_dropped() {
        let search = Search {
            root_tags: Some(vec!["a3f1".to_string(), String::new()]),
            file_tags: Some(vec![String::new()]),
            ..Default::default()
        };

        let (_, root, file, _, _, _, _) = search.into_tuple();

        assert_eq!(root, vec!["a3f1".to_string()]);
        assert!(file.is_empty());
    }
}
