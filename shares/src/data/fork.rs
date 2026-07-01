use serde::{Deserialize, Serialize};
use validr::*;

/// Body for `POST /api/shares/{file_id}/fork`. The client has already
/// decrypted the source file with their existing per-user wrap,
/// generated a fresh symmetric key, and re-encrypted the content; the
/// fields below describe the new file the server should create on the
/// caller's drive. Chunk upload follows via the standard
/// `POST /api/storage/{file_id}` route using the returned id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForkBody {
    pub new_file_id: Option<String>,
    pub encrypted_metadata: Option<String>,
    pub encrypted_thumbnail: Option<String>,
    pub name_hash: Option<String>,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub chunks: Option<i64>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
    pub cipher: Option<String>,
    pub encrypted_key: Option<String>,
    pub search_tokens_hashed: Option<Vec<String>>,
    pub event_signature: Option<String>,
    pub timestamp: Option<i64>,
}

impl Validation for ForkBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(new_file_id),
            rule_required!(encrypted_metadata),
            rule_required!(name_hash),
            rule_required!(mime),
            rule_required!(encrypted_key),
            rule_required!(event_signature),
            rule_required!(timestamp),
            Rule::new("size", |obj: &ForkBody, error| {
                if matches!(obj.mime.as_deref(), Some("dir")) {
                    if obj.size.is_some() {
                        error.add("not_for_dir");
                    }
                    return;
                }
                match obj.size {
                    Some(v) if v > 0 => {}
                    Some(_) => error.add("min:1"),
                    None => error.add("required"),
                }
            }),
            Rule::new("chunks", |obj: &ForkBody, error| {
                if matches!(obj.mime.as_deref(), Some("dir")) {
                    if obj.chunks.is_some() {
                        error.add("not_for_dir");
                    }
                    return;
                }
                match obj.chunks {
                    Some(v) if v > 0 => {}
                    Some(_) => error.add("min:1"),
                    None => error.add("required"),
                }
            }),
        ]
    }
}
