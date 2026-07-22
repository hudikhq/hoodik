use cryptfns::tokenizer::Token;
use entity::{option_string_to_uuid, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Search {
    pub dir_id: Option<String>,
    /// Plaintext query. Only clients that predate client-side query hashing
    /// send this; it is tokenized here and ignored entirely when
    /// `search_tokens_hashed` is present.
    pub search: Option<String>,
    /// Search tokens in `"{sha256-hex}:{weight}"` form, tokenized and hashed
    /// on the client — the same shape the create and rename routes accept —
    /// so the plaintext query never reaches the server.
    pub search_tokens_hashed: Option<Vec<String>>,
    pub limit: Option<u64>,
    pub skip: Option<u64>,
    pub editable: Option<bool>,
    /// Withhold `encrypted_thumbnail` from the results and report only
    /// `has_thumbnail`. Absent means full rows — the compatible default
    /// for older clients.
    pub compact: Option<bool>,
}

impl Validation for Search {}

pub type SearchData = (Option<Uuid>, Vec<Token>, Option<u64>, Option<u64>, Option<bool>);

impl Search {
    pub fn into_tuple(self) -> SearchData {
        let tokens = match self.search_tokens_hashed {
            Some(hashed) => cryptfns::tokenizer::from_vec(hashed).unwrap_or_default(),
            None => cryptfns::tokenizer::into_hashed_tokens(&self.search.unwrap_or_default())
                .unwrap_or_default(),
        };

        (
            option_string_to_uuid(self.dir_id),
            tokens,
            self.limit,
            self.skip,
            self.editable,
        )
    }
}
