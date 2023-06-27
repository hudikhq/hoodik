//! Take the data to rename a file or a folder,
//! the data needs to be encrypted with the file key before
//! it is sent. And a new name_hash needs to be generated.
use ::error::AppResult;
use chrono::Utc;
use entity::{files::ActiveModel as ActiveModelFile, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rename {
    /// Name of the file hashed so we can guard
    /// against duplicate files in directories
    pub name_hash: Option<String>,
    /// File name encrypted with the AES file key
    pub encrypted_name: Option<String>,
    /// Tokens by which this file will be searchable broken down
    /// into tokens using the tokenizing methods
    pub search_tokens_hashed: Option<Vec<String>>,
}

impl Validation for Rename {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(name_hash), rule_required!(encrypted_name)]
    }
}

impl Rename {
    pub fn into_active_model(self, id: Uuid) -> AppResult<(ActiveModelFile, Vec<String>, String)> {
        let data = self.validate()?;
        let now = Utc::now().naive_utc();
        let name_hash = data.name_hash.unwrap();

        Ok((
            ActiveModelFile {
                id: ActiveValue::Set(id),
                name_hash: ActiveValue::Set(name_hash.clone()),
                encrypted_name: ActiveValue::Set(data.encrypted_name.unwrap()),
                file_modified_at: ActiveValue::Set(now.timestamp()),
                ..Default::default()
            },
            data.search_tokens_hashed.unwrap_or_default(),
            name_hash,
        ))
    }
}
