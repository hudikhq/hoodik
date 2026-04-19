//! Toggle the `editable` flag on an existing file.
//! Only the owner can convert a regular file into an editable note (or back).
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetEditable {
    pub editable: Option<bool>,
}

impl Validation for SetEditable {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(editable)]
    }
}
