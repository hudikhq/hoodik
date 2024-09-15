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
    pub mime: String,
    pub size: Option<i64>,
    pub chunks: Option<i64>,
    pub chunks_stored: Option<i64>,
    pub file_id: Option<Uuid>,
    pub file_modified_at: i64,
    pub created_at: i64,
    pub finished_upload_at: Option<i64>,
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
            mime: file.mime,
            size: file.size,
            chunks: file.chunks,
            chunks_stored: file.chunks_stored,
            file_id: file.file_id,
            file_modified_at: file.file_modified_at,
            created_at: file.created_at,
            finished_upload_at: file.finished_upload_at,
            is_new: false,
            uploaded_chunks: None,
            link,
        })
    }
}
