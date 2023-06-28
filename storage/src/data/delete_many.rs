//! Run a query to delete many files and folders recursively
use ::error::AppResult;
use entity::Uuid;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteMany {
    /// List of file and folder ids to be deleted recursively
    pub ids: Option<Vec<Uuid>>,
}

impl Validation for DeleteMany {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![Rule::new("ids", |obj: &DeleteMany, error| {
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

impl DeleteMany {
    pub fn into_value(self) -> AppResult<Vec<Uuid>> {
        let data = self.validate()?;

        Ok(data.ids.unwrap_or_default())
    }
}
