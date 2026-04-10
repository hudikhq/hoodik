use cryptfns::tokenizer::Token;
use entity::{option_string_to_uuid, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Search {
    pub dir_id: Option<String>,
    pub search: Option<String>,
    pub limit: Option<u64>,
    pub skip: Option<u64>,
    pub editable: Option<bool>,
}

impl Validation for Search {}

pub type SearchData = (Option<Uuid>, String, Vec<Token>, Option<u64>, Option<u64>, Option<bool>);

impl Search {
    pub fn into_tuple(self) -> SearchData {
        let search = self.search.unwrap_or_default();

        let search_tokens_hashed =
            cryptfns::tokenizer::into_hashed_tokens(&search).unwrap_or_default();

        (
            option_string_to_uuid(self.dir_id),
            search,
            search_tokens_hashed,
            self.limit,
            self.skip,
            self.editable,
        )
    }
}
