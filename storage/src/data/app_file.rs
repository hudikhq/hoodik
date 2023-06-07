use chrono::NaiveDateTime;
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
    pub mime: String,
    pub size: Option<i64>,
    pub chunks: Option<i32>,
    pub chunks_stored: Option<i32>,
    pub file_id: Option<Uuid>,
    pub file_created_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub finished_upload_at: Option<NaiveDateTime>,
    pub is_new: bool,
    pub uploaded_chunks: Option<Vec<i32>>,
    pub link: Option<links::Model>,
}

impl IntoFilename for AppFile {
    fn filename(&self) -> AppResult<Filename> {
        if self.is_dir() {
            return Err(Error::BadRequest(
                "cannot_get_filename_from_dir".to_string(),
            ));
        }

        Ok(Filename::new(self.created_at, self.id))
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
            encrypted_name: file.encrypted_name,
            encrypted_thumbnail: file.encrypted_thumbnail,
            mime: file.mime,
            size: file.size,
            chunks: file.chunks,
            chunks_stored: file.chunks_stored,
            file_id: file.file_id,
            file_created_at: file.file_created_at,
            created_at: file.created_at,
            finished_upload_at: file.finished_upload_at,
            is_new: false,
            uploaded_chunks: None,
            link,
        })
    }
}
