//! Repository module for running query operations on files that will automatically filter
//! them for only the files where the user has the file shared with him.

use entity::{users, ConnectionTrait, Uuid};
use error::{AppResult, Error};

use crate::data::app_file::AppFile;

use super::Repository;

pub struct Query<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    user: &'repository users::Model,
}

impl<'repository, T> Query<'repository, T>
where
    T: ConnectionTrait,
{
    pub fn new(
        repository: &'repository Repository<'repository, T>,
        user: &'repository users::Model,
    ) -> Self {
        Self { repository, user }
    }

    /// Get any kind of file for the user
    pub async fn get(&self, id: Uuid) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.user.id).await?;

        Ok(file)
    }

    /// Alias to get the file metadata for the user
    pub async fn file(&self, id: Uuid) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.user.id).await?;

        if file.is_dir() {
            return Err(Error::BadRequest("file_not_found".to_string()));
        }

        Ok(file)
    }

    /// Alias to get directory metadata for the user
    pub async fn dir(&self, id: Uuid) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.user.id).await?;

        if file.is_file() {
            return Err(Error::NotFound("directory_not_found".to_string()));
        }

        Ok(file)
    }
}
