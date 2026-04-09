//! Toggle the `editable` flag on an existing file.
//! Only the owner can convert a regular file into an editable note (or back).
use ::error::AppResult;
use entity::{files::ActiveModel as ActiveModelFile, ActiveValue, Uuid};
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

impl SetEditable {
    pub fn into_active_model(self, id: Uuid) -> AppResult<ActiveModelFile> {
        let data = self.validate()?;

        Ok(ActiveModelFile {
            id: ActiveValue::Set(id),
            editable: ActiveValue::Set(data.editable.unwrap()),
            ..Default::default()
        })
    }
}
