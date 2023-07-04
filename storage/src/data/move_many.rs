//! Move many files and folders into a new parent folder
use ::error::AppResult;
use entity::Uuid;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveMany {
    /// List of file and folder ids to be moved
    pub ids: Option<Vec<Uuid>>,
    /// Destination folder id (empty for root)
    pub file_id: Option<Uuid>,
}

impl Validation for MoveMany {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![Rule::new("ids", |obj: &MoveMany, error| {
            if let Some(ids) = obj.ids.as_ref() {
                if ids.is_empty() {
                    error.add("required")
                }
            } else {
                error.add("required")
            }
        })]
    }
}

impl MoveMany {
    pub fn into_value(self) -> AppResult<(Vec<Uuid>, Option<Uuid>)> {
        let data = self.validate()?;

        Ok((data.ids.unwrap_or_default(), data.file_id))
    }
}
