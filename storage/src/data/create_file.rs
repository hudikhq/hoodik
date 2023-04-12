//! Create a file data, this is used to either initiate a new file upload,
//! or to query the data of already started file to get all the required information
//! required to continue the upload using the resumable upload.
//!
//! Encrypted key might be discarded in case the file is already started with some other key,
//! the resume will have to continue with the returned key in that case.
//!
//! If not, the file will be corrupted and we have no way of knowing if that is the case.
use ::error::AppResult;
use chrono::Utc;
use entity::{files::ActiveModel as ActiveModelFile, ActiveValue};
use serde::{Deserialize, Serialize};
use validr::*;

use crate::CHUNK_SIZE_BYTES;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateFile {
    /// Name of the file, encrypted with the user's public key
    pub name_enc: Option<String>,
    /// Tokens by which this file will be searchable broken down
    /// into tokens using the tokenizing methods
    pub search_tokens_hashed: Option<Vec<String>>,
    /// Stringified JSON of the: { "salt": string; "iv": string; "passphrase": string; }
    pub encrypted_key: Option<String>,
    /// Original file MD5 checksum
    pub checksum: Option<String>,
    /// Mime type of the file or "dir" for directory
    pub mime: Option<String>,
    /// Total size of the file
    pub size: Option<i64>,
    /// Total number of chunks, must be ceil(size / CHUNK_SIZE_BYTES)
    pub chunks: Option<i32>,
    /// ID of the directory the file is located in (directories are files too)
    pub file_id: Option<i32>,
    /// Date of the file creation from the disk, if not provided we set it to now
    pub file_created_at: Option<String>,
}

impl Validation for CreateFile {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(name_enc),
            rule_required!(encrypted_key),
            rule_required!(checksum),
            rule_required!(mime),
            Rule::new("size", |obj: &CreateFile, error| {
                let dir_mime = Some("dir".to_string());

                if obj.mime.as_ref() == dir_mime.as_ref() {
                    if obj.size.is_some() {
                        return error.add("not_for_dir");
                    }

                    return;
                }

                if let Some(v) = obj.size {
                    if v <= 0 {
                        return error.add("min:1");
                    }
                }
            }),
            // We do this validation to make sure the frontend knows exactly
            // how many chunks to upload. We could simply assume this is known
            // by the frontend, but is better because the actual chunk
            // sizes are not exactly 1MB because of the encryption overhead.
            Rule::new("chunks", |obj: &CreateFile, error| {
                let dir_mime = Some("dir".to_string());

                if obj.mime.as_ref() == dir_mime.as_ref() {
                    if obj.chunks.is_some() {
                        return error.add("not_for_dir");
                    }

                    return;
                }

                if let Some(v) = obj.chunks {
                    if v <= 0 {
                        return error.add("min:1");
                    }

                    let size = obj.size.unwrap_or(1);
                    let expected_chunks: f64 = size as f64 / CHUNK_SIZE_BYTES as f64;
                    let mut expected_chunks_u64 = size as u64 / CHUNK_SIZE_BYTES;

                    if (expected_chunks - expected_chunks_u64 as f64) > 0.0 {
                        expected_chunks_u64 += 1;
                    }

                    if v as u64 != expected_chunks_u64 {
                        return error.add(
                            format!("invalid_chunks_expected:{}", expected_chunks_u64).as_str(),
                        );
                    }
                }
            }),
            Rule::new("file_created_at", |obj: &CreateFile, error| {
                if let Some(v) = &obj.file_created_at {
                    if util::datetime::parse_into_naive_datetime(v, Some("file_created_at"))
                        .is_err()
                    {
                        return error.add("invalid_date");
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![]
    }
}

impl CreateFile {
    pub fn into_active_model(self) -> AppResult<(ActiveModelFile, String)> {
        let data = self.validate()?;
        let now = Utc::now().naive_utc();

        let chunks_stored = if data.mime != Some("dir".to_string()) {
            Some(0)
        } else {
            None
        };

        Ok((
            ActiveModelFile {
                id: ActiveValue::NotSet,
                name_enc: ActiveValue::Set(data.name_enc.unwrap()),
                checksum: ActiveValue::Set(data.checksum.clone().unwrap()),
                mime: ActiveValue::Set(data.mime.unwrap()),
                size: ActiveValue::Set(data.size),
                chunks: ActiveValue::Set(data.chunks),
                chunks_stored: ActiveValue::Set(chunks_stored),
                file_id: ActiveValue::Set(data.file_id),
                file_created_at: ActiveValue::Set(
                    data.file_created_at
                        .map(|i| {
                            util::datetime::parse_into_naive_datetime(&i, Some("file_created_at"))
                                .unwrap()
                        })
                        .unwrap_or(now),
                ),
                created_at: ActiveValue::Set(now),
                finished_upload_at: ActiveValue::Set(None),
            },
            data.encrypted_key.unwrap(),
        ))
    }
}