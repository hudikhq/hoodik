//! Data struct for replacing the content of an editable file.
//! Used when saving updated markdown content via `PUT /api/storage/{file_id}/content`.

use ::error::AppResult;
use chrono::Utc;
use entity::{files::ActiveModel as ActiveModelFile, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplaceContent {
    /// New total size of the file content
    pub size: Option<i64>,
    /// New total number of chunks
    pub chunks: Option<i64>,
    /// Cipher used to encrypt the new content (keeps existing if not provided)
    pub cipher: Option<String>,
    /// Updated encrypted file name (keeps existing if not provided)
    pub encrypted_name: Option<String>,
    /// Updated encrypted thumbnail (keeps existing if not provided)
    pub encrypted_thumbnail: Option<String>,
    /// Updated search tokens from content tokenization
    pub search_tokens_hashed: Option<Vec<String>>,
}

impl Validation for ReplaceContent {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(size),
            rule_required!(chunks),
            Rule::new("size", |obj: &ReplaceContent, error| {
                if let Some(v) = obj.size {
                    if v <= 0 {
                        error.add("min:1")
                    }
                }
            }),
            Rule::new("chunks", |obj: &ReplaceContent, error| {
                if let Some(v) = obj.chunks {
                    if v <= 0 {
                        error.add("min:1")
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![]
    }
}

impl ReplaceContent {
    pub fn into_active_model(self, id: Uuid) -> AppResult<(ActiveModelFile, Vec<String>)> {
        let data = self.validate()?;
        let now = Utc::now().naive_utc();

        let mut active_model = ActiveModelFile {
            id: ActiveValue::Set(id),
            size: ActiveValue::Set(data.size),
            chunks: ActiveValue::Set(data.chunks),
            chunks_stored: ActiveValue::Set(Some(0)),
            finished_upload_at: ActiveValue::Set(None),
            file_modified_at: ActiveValue::Set(now.and_utc().timestamp()),
            // Clear stale hashes — new hashes will be computed after re-upload
            md5: ActiveValue::Set(None),
            sha1: ActiveValue::Set(None),
            sha256: ActiveValue::Set(None),
            blake2b: ActiveValue::Set(None),
            ..Default::default()
        };

        if let Some(cipher) = data.cipher {
            active_model.cipher = ActiveValue::Set(cipher);
        }

        if let Some(encrypted_name) = data.encrypted_name {
            active_model.encrypted_name = ActiveValue::Set(encrypted_name);
        }

        if let Some(encrypted_thumbnail) = data.encrypted_thumbnail {
            active_model.encrypted_thumbnail = ActiveValue::Set(Some(encrypted_thumbnail));
        }

        Ok((active_model, data.search_tokens_hashed.unwrap_or_default()))
    }
}
