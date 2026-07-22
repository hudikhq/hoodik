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
    /// Comma-separated whitelist of row fields to include in the
    /// response. Absent means full rows — the compatible default for
    /// clients that predate the parameter.
    pub attributes: Option<String>,
}

impl Validation for Query {}
