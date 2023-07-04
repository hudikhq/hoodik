use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Query {
    pub dir_id: Option<String>,
    pub order: Option<String>,
    pub order_by: Option<String>,
    pub dirs_only: Option<bool>,
    pub is_owner: Option<bool>,
}

impl Validation for Query {}
