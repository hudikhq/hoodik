use entity::{files, links, users, DbErr, FromQueryResult, QueryResult, Uuid};
use error::AppResult;
use fs::{prelude::Filename, IntoFilename};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppLink {
    pub id: Uuid,
    pub file_id: Uuid,
    pub owner_id: Uuid,
    pub owner_email: String,
    pub owner_pubkey: String,
    /// Owner's key algorithm (`"rsa"` or `"curve25519"`), so the recipient
    /// verifies `signature` with the scheme the owner actually signed with.
    pub owner_key_type: String,
    pub file_size: Option<i64>,
    pub file_mime: String,
    /// Signature that the user created when the link was initially created.
    ///
    /// The signature is made using a shared file_id
    pub signature: String,
    /// Number of times the file has been downloaded,
    /// this counts every attempt to download the file,
    /// it doesn't wait for the file to be downloaded in order to increment.
    pub downloads: i32,
    /// Name of the file encrypted with the link key. If the file is
    /// renamed after the link is created, that change won't be reflected
    /// in the link.
    pub encrypted_name: String,
    /// Link AES key encrypted with the user's public RSA key.
    /// So the owner of the link can retrieve the link key directly
    /// from the database entry, everyone else must have it shared with them.
    pub encrypted_link_key: String,
    /// If the file has a thumbnail it is encrypted with the link key.
    /// Sent by default; listings asked for `compact` leave it in the
    /// database and clients fetch the blob per link from the metadata
    /// route.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_thumbnail: Option<String>,
    /// Whether this link carries a thumbnail, computed in SQL so it stays
    /// accurate when `compact` withholds the blob itself.
    #[serde(default)]
    pub has_thumbnail: bool,
    /// The file's content key encrypted with the link key. The recipient
    /// holds the link key (URL fragment), unwraps this to the content key,
    /// and decrypts the ciphertext chunks in the browser — the server never
    /// sees either key in the clear.
    pub encrypted_file_key: Option<String>,
    pub created_at: i64,
    pub file_modified_at: i64,
    /// Cipher used to encrypt the file chunks (e.g. "ascon128a", "aegis128l").
    /// Comes from the joined `files` table, not from the `links` table itself.
    pub file_cipher: String,
    /// Active version of the file's chunks. Public link downloads always
    /// stream the active version — a save in progress on the owner's side
    /// is invisible to the link recipient until it commits.
    pub file_active_version: i32,
    /// Mirrors `files.editable`. Drives the same layout split as the
    /// owner-facing routes: editable → versioned `v{N}/` path, otherwise
    /// the legacy flat layout.
    pub file_editable: bool,
    /// Date when the link will expire, automated cron job
    /// will periodically empty out the expired links of all the
    /// file metadata and encrypted file key.
    pub expires_at: Option<i64>,
}

impl AppLink {
    /// Let us know if the link has expired so we can prevent it from being downloaded.
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();

        self.expires_at
            .map(|expires_at| expires_at < now)
            .unwrap_or(false)
    }
}

impl FromQueryResult for AppLink {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let link = links::Model::from_query_result(res, "link")?;
        let user = users::Model::from_query_result(res, "user")?;
        let file = files::Model::from_query_result(res, "file")?;

        Ok(Self {
            id: link.id,
            file_id: file.id,
            file_size: file.size,
            file_mime: file.mime,
            signature: link.signature,
            downloads: link.downloads,
            encrypted_name: link.encrypted_name,
            encrypted_link_key: link.encrypted_link_key,
            // Computed in SQL by the listing selector; the fallback covers
            // the single-link lookups that don't project it.
            has_thumbnail: res
                .try_get::<bool>("", "has_thumbnail")
                .unwrap_or_else(|_| link.encrypted_thumbnail.is_some()),
            encrypted_thumbnail: link.encrypted_thumbnail,
            encrypted_file_key: link.encrypted_file_key,
            created_at: link.created_at,
            file_modified_at: file.created_at,
            file_cipher: file.cipher,
            file_active_version: file.active_version,
            file_editable: file.editable,
            expires_at: link.expires_at,
            owner_id: user.id,
            owner_email: user.email,
            owner_pubkey: user.pubkey,
            owner_key_type: user.key_type,
        })
    }
}

impl IntoFilename for AppLink {
    fn filename(&self) -> AppResult<Filename> {
        Ok(Filename::new(self.file_id).with_timestamp(self.file_modified_at))
    }
}
