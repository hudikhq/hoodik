//! Take the data to rename a file or a folder,
//! the data needs to be encrypted with the file key before
//! it is sent. And a new name_hash needs to be generated.
use ::error::{AppResult, Error};
use chrono::Utc;
use entity::{
    file_tokens::SearchTags, files::ActiveModel as ActiveModelFile, ActiveValue, Uuid,
};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rename {
    /// Name of the file hashed so we can guard
    /// against duplicate files in directories
    pub name_hash: Option<String>,
    /// File name encrypted with the AES file key
    pub encrypted_name: Option<String>,
    /// Tags for the new name under the caller's account-wide search key.
    /// Absent when the caller is an editor rather than the owner — they hold
    /// the file key but not the owner's root key, and the writer leaves the
    /// scope it was not given alone.
    pub search_tokens_root: Option<Vec<String>>,
    /// Tags for the new name under the file's own key.
    pub search_tokens_file: Option<Vec<String>>,
}

impl Validation for Rename {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(name_hash), rule_required!(encrypted_name)]
    }
}

impl Rename {
    pub fn into_active_model(self, id: Uuid) -> AppResult<(ActiveModelFile, SearchTags, String)> {
        let data = self.validate()?;

        // Refuse a pre-keyed `sha256(name)` the same way create does, so an old
        // client can't put the reversible digest back through a rename.
        if let Some(hash) = data.name_hash.as_deref() {
            if cryptfns::search::is_legacy_name_hash(hash) {
                return Err(Error::UpgradeRequired(
                    "client_too_old_for_search".to_string(),
                ));
            }
        }

        let now = Utc::now().naive_utc();
        let name_hash = data.name_hash.unwrap();

        Ok((
            ActiveModelFile {
                id: ActiveValue::Set(id),
                name_hash: ActiveValue::Set(name_hash.clone()),
                encrypted_name: ActiveValue::Set(data.encrypted_name.unwrap()),
                file_modified_at: ActiveValue::Set(now.and_utc().timestamp()),
                ..Default::default()
            },
            SearchTags::new(data.search_tokens_root, data.search_tokens_file),
            name_hash,
        ))
    }
}
