use entity::{option_string_to_uuid, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Search {
    pub dir_id: Option<String>,
    pub search_tokens_hashed: Option<Vec<String>>,
    pub limit: Option<u64>,
    pub skip: Option<u64>,
}

impl Validation for Search {}

impl Search {
    pub fn into_tuple(self) -> (Option<Uuid>, Vec<String>, Option<u64>, Option<u64>) {
        (
            option_string_to_uuid(self.dir_id),
            self.search_tokens_hashed.unwrap_or_default(),
            self.limit,
            self.skip,
        )
    }
}
