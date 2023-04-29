use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Query {
    pub dir_id: Option<String>,
    pub order: Option<String>,
    pub order_by: Option<String>,
}

impl Validation for Query {}
