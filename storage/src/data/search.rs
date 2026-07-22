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
    /// A file content hash (md5/sha1/sha256/blake2b hex) to match verbatim
    /// against the stored hash columns, letting a user find a file by the
    /// digest of its bytes. Computed from the file's own content on the
    /// client, and the server already stores all four, so it carries nothing
    /// the server does not have.
    pub hash: Option<String>,
    pub limit: Option<u64>,
    pub skip: Option<u64>,
    pub editable: Option<bool>,
    /// Withhold `encrypted_thumbnail` from the results and report only
    /// `has_thumbnail`. Absent means full rows — the compatible default
    /// for older clients.
    pub compact: Option<bool>,
}

impl Validation for Search {}

pub type SearchData = (
    Option<Uuid>,
    Option<String>,
    Vec<Token>,
    Option<u64>,
    Option<u64>,
    Option<bool>,
);

impl Search {
    pub fn into_tuple(self) -> SearchData {
        let (hash, tokens) = match self.search_tokens_hashed {
            Some(hashed) => (
                self.hash,
                cryptfns::tokenizer::from_vec(hashed).unwrap_or_default(),
            ),
            // Legacy clients send one plaintext string that serves as both the
            // term to tokenize and the hash to match, which is how the route
            // behaved before tokenization moved to the client.
            None => {
                let search = self.search.unwrap_or_default();
                let tokens =
                    cryptfns::tokenizer::into_hashed_tokens(&search).unwrap_or_default();

                (Some(search), tokens)
            }
        };

        (
            option_string_to_uuid(self.dir_id),
            hash.filter(|h| !h.is_empty()),
            tokens,
            self.limit,
            self.skip,
            self.editable,
        )
    }
}
