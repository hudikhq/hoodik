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
use entity::{files::ActiveModel as ActiveModelFile, option_string_to_uuid, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use util::rule_required_if;
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateFile {
    /// File key encrypted with users RSA key
    pub encrypted_key: Option<String>,
    /// Name of the file hashed so we can guard
    /// against duplicate files in directories
    pub name_hash: Option<String>,
    /// File name encrypted with the AES file key
    pub encrypted_name: Option<String>,
    /// File thumbnail encrypted with the AES file key
    pub encrypted_thumbnail: Option<String>,
    /// Tokens by which this file will be searchable broken down
    /// into tokens using the tokenizing methods
    pub search_tokens_hashed: Option<Vec<String>>,
    /// Mime type of the file or "dir" for directory
    pub mime: Option<String>,
    /// Total size of the file
    pub size: Option<i64>,
    /// Total number of chunks, no limitations, frontend can decide
    pub chunks: Option<i64>,
    /// ID of the directory the file is located in (directories are files too)
    pub file_id: Option<String>,
    /// Date of the file creation from the disk, if not provided we set it to now
    pub file_modified_at: Option<String>,
    /// MD5 hash of the unencrypted file
    pub md5: Option<String>,
    /// SHA1 hash of the unencrypted file
    pub sha1: Option<String>,
    /// SHA256 hash of the unencrypted file
    pub sha256: Option<String>,
    /// BLAKE2B hash of the unencrypted file
    pub blake2b: Option<String>,
}

impl Validation for CreateFile {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(encrypted_key),
            rule_required!(name_hash),
            rule_required!(encrypted_name),
            rule_required!(mime),
            rule_required_if!(md5, |_v, obj: &CreateFile| obj.mime.as_ref() != Some(&"dir".to_string())),
            rule_required_if!(sha1, |_v, obj: &CreateFile| obj.mime.as_ref() != Some(&"dir".to_string())),
            rule_required_if!(sha256, |_v, obj: &CreateFile| obj.mime.as_ref() != Some(&"dir".to_string())),
            rule_required_if!(blake2b, |_v, obj: &CreateFile| obj.mime.as_ref() != Some(&"dir".to_string())),
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
                        error.add("min:1")
                    }
                } else {
                    error.add("required")
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
                        error.add("min:1");
                    }

                    // We won't validate the size of each chunk because we won't allow that
                    // flexibility from the backend
                    // let size = obj.size.unwrap_or(1);
                    // let expected_chunks: f64 = size as f64 / crate::CHUNK_SIZE_BYTES as f64;
                    // let mut expected_chunks_u64 = size as u64 / crate::CHUNK_SIZE_BYTES;

                    // if (expected_chunks - expected_chunks_u64 as f64) > 0.0 {
                    //     expected_chunks_u64 += 1;
                    // }

                    // if v as u64 != expected_chunks_u64 {
                    //     error.add(
                    //         format!("invalid_chunks_expected:{}", expected_chunks_u64).as_str(),
                    //     )
                    // }
                } else {
                    error.add("required")
                }
            }),
            Rule::new("file_modified_at", |obj: &CreateFile, error| {
                if let Some(v) = &obj.file_modified_at {
                    if util::datetime::parse_into_naive_datetime(v, Some("file_modified_at"))
                        .is_err()
                    {
                        error.add("invalid_date")
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![]
    }
}

pub type CreateFileData = (ActiveModelFile, String, Vec<String>, i64, Option<Uuid>);

impl CreateFile {
    pub fn into_active_model(self) -> AppResult<CreateFileData> {
        let data = self.validate()?;
        let now = Utc::now().naive_utc();

        let chunks_stored = if data.mime != Some("dir".to_string()) {
            Some(0)
        } else {
            None
        };

        let file_id = option_string_to_uuid(data.file_id.clone());

        Ok((
            ActiveModelFile {
                id: ActiveValue::Set(entity::Uuid::new_v4()),
                name_hash: ActiveValue::Set(data.name_hash.unwrap()),
                encrypted_name: ActiveValue::Set(data.encrypted_name.unwrap()),
                encrypted_thumbnail: ActiveValue::Set(data.encrypted_thumbnail),
                mime: ActiveValue::Set(data.mime.unwrap()),
                size: ActiveValue::Set(data.size),
                chunks: ActiveValue::Set(data.chunks),
                chunks_stored: ActiveValue::Set(chunks_stored),
                file_id: ActiveValue::Set(option_string_to_uuid(data.file_id)),
                file_modified_at: ActiveValue::Set(
                    data.file_modified_at
                        .map(|i| {
                            util::datetime::parse_into_naive_datetime(&i, Some("file_modified_at"))
                                .unwrap()
                        })
                        .unwrap_or(now)
                        .and_utc().timestamp(),
                ),
                md5: ActiveValue::Set(data.md5),
                sha1: ActiveValue::Set(data.sha1),
                sha256: ActiveValue::Set(data.sha256),
                blake2b: ActiveValue::Set(data.blake2b),
                created_at: ActiveValue::Set(now.and_utc().timestamp()),
                finished_upload_at: ActiveValue::Set(None),
            },
            data.encrypted_key.unwrap(),
            data.search_tokens_hashed.unwrap_or_default(),
            data.size.unwrap_or(0),
            file_id,
        ))
    }
}
