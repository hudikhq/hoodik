use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Query {
    pub dir_id: Option<String>,
    pub order: Option<String>,
    pub order_by: Option<String>,
    pub dirs_only: Option<bool>,
    pub is_owner: Option<bool>,
    pub editable: Option<bool>,
    /// Withhold `encrypted_thumbnail` from the rows and report only
    /// `has_thumbnail`. Absent means full rows — the compatible default
    /// for clients that predate the parameter.
    pub compact: Option<bool>,
}

impl Validation for Query {}
