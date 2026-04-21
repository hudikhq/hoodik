//! Data struct for replacing the content of an editable file.
//!
//! Drives the two-phase versioned-chunks flow:
//!
//! 1. Client sends `PUT /api/storage/{file_id}/content` with the new size,
//!    chunk count, and (optionally) updated metadata.
//! 2. Server allocates a new pending version and stores `pending_chunks` /
//!    `pending_size`. **No on-disk side effects** — the active version's
//!    chunks remain untouched and readers continue to see the previous
//!    content.
//! 3. Client uploads the new chunks. Each one lands under
//!    `{file_id}/v{pending_version}/`.
//! 4. When `chunks_stored == pending_chunks`, the upload route auto-fires
//!    `Manage::finish`, which atomically swaps `active_version =
//!    pending_version` inside a transaction.
//!
//! `force = true` is the recovery escape hatch when a previous edit died
//! mid-way: it overrides the 409 conflict, abandons the orphaned pending
//! directory on disk, and starts a fresh pending version. Without `force`
//! the second `replaceContent` returns 409 `UploadInProgress` — that's the
//! safety net against accidental concurrent edits from a second device.
//!
//! Note: `cipher` is intentionally NOT in this payload. Changing the cipher
//! mid-edit would break decryption of the active version's chunks. If a
//! cipher swap is ever needed, it has to land via a separate, full file
//! re-upload path.

use ::error::AppResult;
use entity::Uuid;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplaceContent {
    /// New total size of the file content.
    pub size: Option<i64>,
    /// New total number of chunks for the upcoming upload.
    pub chunks: Option<i64>,
    /// Updated encrypted file name (kept as the display name across edits;
    /// only swapped if the client explicitly provides a new value).
    pub encrypted_name: Option<String>,
    /// Updated encrypted thumbnail (same opt-in semantics as `encrypted_name`).
    pub encrypted_thumbnail: Option<String>,
    /// Updated search tokens computed from the new plaintext content.
    pub search_tokens_hashed: Option<Vec<String>>,
    /// When true, abandon any in-progress edit (`pending_version`) and
    /// start fresh. The repository purges the orphaned pending directory
    /// on disk and allocates a brand-new pending version. Without this
    /// flag, the request 409s when a pending upload exists.
    pub force: Option<bool>,
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

/// The validated, normalized form passed to `Manage::replace_content`.
/// Consumes [`ReplaceContent`] so callers can't accidentally use the raw
/// payload after validation.
pub struct ValidatedReplaceContent {
    pub _id: Uuid,
    pub size: i64,
    pub chunks: i64,
    pub encrypted_name: Option<String>,
    pub encrypted_thumbnail: Option<String>,
    pub search_tokens_hashed: Vec<String>,
    pub force: bool,
}

impl ReplaceContent {
    pub fn validate_into(self, id: Uuid) -> AppResult<ValidatedReplaceContent> {
        let data = self.validate()?;
        Ok(ValidatedReplaceContent {
            _id: id,
            // `validate()` enforces these via `rule_required!` — unwraps
            // are unreachable past that point.
            size: data.size.unwrap(),
            chunks: data.chunks.unwrap(),
            encrypted_name: data.encrypted_name,
            encrypted_thumbnail: data.encrypted_thumbnail,
            search_tokens_hashed: data.search_tokens_hashed.unwrap_or_default(),
            force: data.force.unwrap_or(false),
        })
    }
}
