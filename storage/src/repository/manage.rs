//! Repository module for manipulating with files in the database
//! this module should only be used by the owner of the file

use chrono::Utc;
use entity::{files, user_files, users, ActiveModelTrait, ActiveValue, ConnectionTrait};
use error::{AppResult, Error};

use crate::data::app_file::AppFile;

use super::Repository;

pub struct Manage<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    owner: &'repository users::Model,
}

impl<'repository, T> Manage<'repository, T>
where
    T: ConnectionTrait,
{
    pub fn new(
        repository: &'repository Repository<'repository, T>,
        owner: &'repository users::Model,
    ) -> Self {
        Self { repository, owner }
    }

    /// Get any kind of file for the owner
    pub async fn get(&self, id: i32) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.owner.id).await?;

        if !file.is_owner {
            return Err(Error::BadRequest("file_not_found".to_string()));
        }

        Ok(file)
    }

    /// Alias to get the file metadata for the owner
    pub async fn file(&self, id: i32) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.owner.id).await?;

        if file.is_dir() {
            return Err(Error::BadRequest("file_not_found".to_string()));
        }

        Ok(file)
    }

    /// Alias to get directory metadata for the owner
    pub async fn dir(&self, id: i32) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.owner.id).await?;

        if file.is_file() {
            return Err(Error::NotFound("directory_not_found".to_string()));
        }

        Ok(file)
    }

    /// Create a file entry in the database and set the owner with the
    /// sent encrypted_key.
    pub async fn create(
        &self,
        create_file: files::ActiveModel,
        encrypted_key: &str,
    ) -> AppResult<AppFile> {
        // Check if the file_id is set, if it is, check if the parent is directory
        // and if the current user is the owner of that directory.
        if let Some(file_id) = create_file.file_id.clone().into_value() {
            if file_id.to_string().as_str() != "NULL" {
                let parent = self.repository.by_id(file_id, self.owner.id).await?;

                if !parent.is_owner || !parent.is_dir() {
                    return Err(Error::BadRequest("parent_directory_not_found".to_string()));
                }
            }
        }

        let file = create_file.insert(self.repository.connection()).await?;
        let user_file = user_files::ActiveModel {
            id: ActiveValue::NotSet,
            file_id: ActiveValue::Set(file.id),
            user_id: ActiveValue::Set(self.owner.id),
            is_owner: ActiveValue::Set(true),
            encrypted_key: ActiveValue::Set(encrypted_key.to_string()),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::NotSet,
        }
        .insert(self.repository.connection())
        .await?;

        Ok(AppFile::from((file, user_file)).is_new(true))
    }

    /// Increment the chunks stored for the given file and mark the file as uploaded
    /// if all the chunks are uploaded.
    pub async fn increment(&self, file: &AppFile) -> AppResult<AppFile> {
        if !file.is_owner || file.user_id != self.owner.id || file.is_dir() {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        let chunks_stored = file
            .chunks_stored
            .ok_or(Error::BadRequest("file_has_no_chunks_stored".to_string()))?;

        let chunks = file
            .chunks
            .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

        let finished_upload_at = if chunks_stored + 1 == chunks {
            Some(Utc::now().naive_utc())
        } else {
            None
        };

        files::ActiveModel {
            id: ActiveValue::Set(file.id),
            chunks_stored: ActiveValue::Set(Some(chunks_stored + 1)),
            finished_upload_at: ActiveValue::Set(finished_upload_at),
            ..Default::default()
        }
        .update(self.repository.connection())
        .await?;

        self.repository.by_id(file.id, file.user_id).await
    }
}
