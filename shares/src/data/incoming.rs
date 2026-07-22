use entity::{user_files, users, DbErr, FromQueryResult, QueryResult, Uuid};
use serde::{Deserialize, Serialize};

/// Recipient-facing view of one incoming share, returned by
/// `GET /api/shares/mine` and `GET /api/shares/mine/by/{user_id}`. The
/// query joins the recipient's `user_files` row to the file's owner row
/// (`is_owner = true`) so the recipient can see who originally owns the
/// content, and to the granter's row via `shared_by_user_id` so the UI can
/// distinguish owner-issued grants from Co-owner re-shares.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingShare {
    pub file_id: Uuid,
    /// MIME type of the shared item. `"dir"` for folders, the file's
    /// MIME for leaves. The recipient UI uses it to decide whether a
    /// row click navigates into the file browser (folder) or opens
    /// the file preview / notes editor (leaf).
    pub mime: String,
    /// Encrypted filename. The recipient decrypts with their wrapped
    /// `encrypted_key` to surface the plaintext name in the Shared-
    /// with-me list; the server never sees the plaintext.
    pub encrypted_name: String,
    /// Encrypted thumbnail (when one was generated at upload time).
    /// Recipients decrypt with the same `encrypted_key`. Sent by
    /// default; listings asked for `compact` leave it in the database and
    /// clients fetch the blob from the storage thumbnail route (their
    /// `user_files` row grants access).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_thumbnail: Option<String>,
    /// Whether the file has a thumbnail, computed in SQL so it stays
    /// accurate when `compact` withholds the blob itself.
    #[serde(default)]
    pub has_thumbnail: bool,
    /// Cipher the file was encrypted with (e.g. `"aegis128l"`). Needed
    /// because the recipient unwraps `encrypted_key` with their RSA
    /// private key and then decrypts the name with this cipher.
    pub cipher: String,
    /// File's `editable` flag from the `files` row. Together with
    /// `share_role` it tells the recipient UI whether a row click
    /// should open the editor (write role + editable file) or the
    /// read-only preview.
    pub editable: bool,
    /// Total bytes, chunk count, and finalize timestamp from the
    /// `files` row. Without these the recipient's DetailsModal and
    /// TableFileRow render "Size: 0 B" and a never-finishing upload
    /// progress bar even when the owner's upload completed long ago.
    pub size: Option<i64>,
    pub chunks: Option<i64>,
    pub chunks_stored: Option<i64>,
    pub finished_upload_at: Option<i64>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
    pub share_role: String,
    pub encrypted_key: String,
    pub created_at: i64,
    pub shared_at: Option<i64>,
    pub owner_id: Uuid,
    pub owner_email: String,
    pub owner_pubkey: String,
    pub owner_key_type: String,
    pub owner_wrapping_pubkey: Option<String>,
    pub owner_pubkey_fingerprint: String,
    pub shared_by_user_id: Option<Uuid>,
    pub shared_by_email: Option<String>,
}

impl FromQueryResult for IncomingShare {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let row = user_files::Model::from_query_result(res, "uf")?;
        let owner = users::Model::from_query_result(res, "owner")?;
        let mime: String = res.try_get("file", "mime").unwrap_or_default();
        let encrypted_name: String = res.try_get("file", "encrypted_name").unwrap_or_default();
        let encrypted_thumbnail: Option<String> = res.try_get("file", "encrypted_thumbnail").ok();
        let cipher: String = res.try_get("file", "cipher").unwrap_or_default();
        let editable: bool = res.try_get("file", "editable").unwrap_or(false);
        let size: Option<i64> = res.try_get("file", "size").ok();
        let chunks: Option<i64> = res.try_get("file", "chunks").ok();
        let chunks_stored: Option<i64> = res.try_get("file", "chunks_stored").ok();
        let finished_upload_at: Option<i64> = res.try_get("file", "finished_upload_at").ok();
        let md5: Option<String> = res.try_get("file", "md5").ok();
        let sha1: Option<String> = res.try_get("file", "sha1").ok();
        let sha256: Option<String> = res.try_get("file", "sha256").ok();
        let blake2b: Option<String> = res.try_get("file", "blake2b").ok();
        let shared_by_email: Option<String> = res.try_get("granter", "email").ok();

        Ok(Self {
            file_id: row.file_id,
            mime,
            encrypted_name,
            has_thumbnail: encrypted_thumbnail.is_some(),
            encrypted_thumbnail,
            cipher,
            editable,
            size,
            chunks,
            chunks_stored,
            finished_upload_at,
            md5,
            sha1,
            sha256,
            blake2b,
            share_role: row.share_role,
            encrypted_key: row.encrypted_key,
            created_at: row.created_at,
            shared_at: row.shared_at,
            owner_id: owner.id,
            owner_email: owner.email,
            owner_pubkey: owner.pubkey,
            owner_key_type: owner.key_type,
            owner_wrapping_pubkey: owner.wrapping_pubkey,
            owner_pubkey_fingerprint: owner.fingerprint,
            shared_by_user_id: row.shared_by_user_id,
            shared_by_email,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IncomingSharePage {
    pub items: Vec<IncomingShare>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IncomingShareQuery {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    /// Withhold `encrypted_thumbnail` from the items and report only
    /// `has_thumbnail`. Absent means full rows — the compatible default
    /// for older clients.
    pub compact: Option<bool>,
}
