use serde::{Deserialize, Serialize};

/// Authentication credentials passed from the host application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub base_url: String,
    pub jwt_token: Option<String>,
    pub refresh_token: Option<String>,
}

/// Metadata about a chunk upload response from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkResponse {
    pub chunks_stored: Option<i64>,
    pub finished_upload_at: Option<i64>,
}

/// Content hashes computed incrementally during upload. Optional fields may be omitted when
/// disabled via [`crate::config::UploadHashOptions`] (JSON omits `None` fields).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blake2b: Option<String>,
}

/// Progress update emitted during transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub file_id: String,
    pub chunk: u64,
    pub total_chunks: u64,
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub file_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}
