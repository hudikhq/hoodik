use serde::{Deserialize, Serialize};

/// Authentication credentials passed from the host application.
///
/// Supports two auth modes:
/// - **JWT**: Set `jwt_token` (+ optional `refresh_token`) for token-based auth (web, CLI).
/// - **Cookie**: Set `cookie` with the raw `Cookie` header value for session-based auth (mobile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub base_url: String,
    pub jwt_token: Option<String>,
    pub refresh_token: Option<String>,
    /// Raw `Cookie` header value (e.g. `"session=abc123"`).  When set, sent as the
    /// `Cookie` HTTP header instead of (or in addition to) the `Authorization` header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookie: Option<String>,
}

/// Where encrypted chunks are downloaded from.
///
/// The two routes differ in more than the path: storage downloads carry the
/// caller's credentials, while public-link downloads are anonymous by design —
/// the server authorises them by link id alone and only ever hands out
/// ciphertext, which the recipient decrypts with the key from the URL fragment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadSource<'a> {
    /// A file owned by or shared with the authenticated user.
    Storage(&'a str),
    /// A file behind a public share link.
    PublicLink(&'a str),
}

impl<'a> DownloadSource<'a> {
    /// The id progress events are keyed by: file id or link id.
    pub fn id(&self) -> &'a str {
        match self {
            Self::Storage(id) | Self::PublicLink(id) => id,
        }
    }

    /// Full URL for one chunk of this source.
    pub fn chunk_url(&self, base_url: &str, chunk: u64) -> String {
        match self {
            Self::Storage(id) => format!("{base_url}/api/storage/{id}?chunk={chunk}"),
            Self::PublicLink(id) => format!("{base_url}/api/links/{id}?chunk={chunk}"),
        }
    }

    /// HTTP method the server expects for chunk downloads from this source.
    pub fn method(&self) -> &'static str {
        match self {
            Self::Storage(_) => "GET",
            Self::PublicLink(_) => "POST",
        }
    }
}

/// Where a transport should go for one chunk, and — the point of the type —
/// what it is allowed to carry when it gets there.
///
/// [`Self::Direct`] holds a URL and nothing else. A transport matching on it
/// has no [`Auth`] in scope, so it cannot attach a session cookie, a bearer
/// token, or a refresh header to a request leaving for the storage bucket.
/// That is not a rule a transport is asked to remember; there is simply
/// nothing there to send.
///
/// Worth spelling out because the alternative was a boolean and an `if` in
/// each of three transports, and the security history of this codebase is
/// that a rule stated in a doc comment held for the entire life of a feature
/// while the code underneath it did the opposite.
#[derive(Debug, Clone, Copy)]
pub enum ChunkTarget<'a> {
    /// This instance's own API, which the caller is authenticated against.
    Api {
        auth: &'a Auth,
        source: DownloadSource<'a>,
    },
    /// A presigned URL at the storage bucket. Credentials would at best be
    /// ignored and at worst be handed to a third party; several S3
    /// implementations also reject a request that carries both a signature
    /// and an `Authorization` header.
    Direct(&'a str),
}

impl<'a> ChunkTarget<'a> {
    /// The id progress events are keyed by. Direct transfers are keyed by the
    /// same id as the API path they replace, so callers thread it through.
    pub fn url(&self, chunk: u64) -> String {
        match self {
            Self::Api { auth, source } => source.chunk_url(&auth.base_url, chunk),
            Self::Direct(url) => (*url).to_string(),
        }
    }

    /// Presigned URLs are signed for one method, and it is never `POST`.
    pub fn method(&self) -> &'static str {
        match self {
            Self::Api { source, .. } => source.method(),
            Self::Direct(_) => "GET",
        }
    }
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
