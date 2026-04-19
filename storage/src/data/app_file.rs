use entity::{files, links, user_files, DbErr, FromQueryResult, QueryResult, Uuid};
use error::{AppResult, Error};
use fs::prelude::{Filename, IntoFilename};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppFile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub is_owner: bool,
    pub encrypted_key: String,
    pub encrypted_name: String,
    pub encrypted_thumbnail: Option<String>,
    pub name_hash: String,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
    pub cipher: String,
    pub editable: bool,
    pub mime: String,
    pub size: Option<i64>,
    pub chunks: Option<i64>,
    pub chunks_stored: Option<i64>,
    pub file_id: Option<Uuid>,
    pub file_modified_at: i64,
    pub created_at: i64,
    pub finished_upload_at: Option<i64>,
    /// The version of chunks that readers should fetch (download/tar/preview).
    /// Always set; defaults to 1.
    pub active_version: i32,
    /// When set, an edit is in flight — chunks are landing into v{pending_version}/.
    /// Readers ignore this; only `replaceContent` and `finish` look at it.
    pub pending_version: Option<i32>,
    /// Total chunks expected for the in-flight upload (NULL when none).
    /// Auto-finalize fires when `chunks_stored == pending_chunks`.
    pub pending_chunks: Option<i64>,
    /// Total size in bytes for the in-flight upload (NULL when none).
    /// Copied to `size` on commit.
    pub pending_size: Option<i64>,
    pub is_new: bool,
    pub uploaded_chunks: Option<Vec<i64>>,
    pub link: Option<links::Model>,
}

impl IntoFilename for AppFile {
    fn filename(&self) -> AppResult<Filename> {
        if self.is_dir() {
            return Err(Error::BadRequest(
                "cannot_get_filename_from_dir".to_string(),
            ));
        }

        Ok(Filename::new(self.id).with_timestamp(self.created_at))
    }
}

impl PartialEq for AppFile {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl AppFile {
    pub fn is_file(&self) -> bool {
        !self.is_dir()
    }

    pub fn is_dir(&self) -> bool {
        &self.mime == "dir"
    }

    pub fn is_new(mut self, is_new: bool) -> Self {
        self.is_new = is_new;

        self
    }

    /// Which version chunk uploads should target. During an edit it's the
    /// pending version; otherwise it's the active version (the file's
    /// initial v1 upload before its first finalize).
    pub fn target_version(&self) -> i32 {
        self.pending_version.unwrap_or(self.active_version)
    }

    /// True if a `replaceContent` is currently in progress for this file.
    pub fn has_pending_upload(&self) -> bool {
        self.pending_version.is_some()
    }

    /// The chunk count the upload counter should be compared against to
    /// decide whether all chunks have landed. During a pending edit this is
    /// `pending_chunks`; otherwise it's the active `chunks` (initial upload
    /// of a brand-new file).
    pub fn target_chunks(&self) -> Option<i64> {
        self.pending_chunks.or(self.chunks)
    }

    /// Whether this file goes through the versioned-chunks layout
    /// (`{uuid}/v{N}/…`) or the legacy flat layout (`{timestamp}-{uuid}.part.N`).
    ///
    /// Only editable files use versioning — the snapshot/swap machinery
    /// exists to support in-place edits. Non-editable files are write-once,
    /// so pushing them through the versioned path would add cost for no
    /// benefit and also trip S3's still-stubbed `_v` methods. Splitting
    /// at the data layer keeps the provider traits dumb dispatchers.
    pub fn use_versioned_layout(&self) -> bool {
        self.editable
    }
}

impl FromQueryResult for AppFile {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let file = files::Model::from_query_result(res, "file")?;
        let user_file = user_files::Model::from_query_result(res, "user_file")?;
        let link = links::Model::from_query_result(res, "link").ok();

        Ok(Self {
            id: file.id,
            user_id: user_file.user_id,
            is_owner: user_file.is_owner,
            encrypted_key: user_file.encrypted_key,
            name_hash: file.name_hash,
            md5: file.md5,
            sha1: file.sha1,
            sha256: file.sha256,
            blake2b: file.blake2b,
            encrypted_name: file.encrypted_name,
            encrypted_thumbnail: file.encrypted_thumbnail,
            cipher: file.cipher,
            editable: file.editable,
            mime: file.mime,
            size: file.size,
            chunks: file.chunks,
            chunks_stored: file.chunks_stored,
            file_id: file.file_id,
            file_modified_at: file.file_modified_at,
            created_at: file.created_at,
            finished_upload_at: file.finished_upload_at,
            active_version: file.active_version,
            pending_version: file.pending_version,
            pending_chunks: file.pending_chunks,
            pending_size: file.pending_size,
            is_new: false,
            uploaded_chunks: None,
            link,
        })
    }
}
