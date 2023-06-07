//! Repository module for manipulating with files in the database
//! this module should only be used by the owner of the file
use std::{cmp::Ordering, fmt::Display, str::FromStr};

use chrono::Utc;
use entity::{
    files, user_files, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait,
    Order, QueryFilter, QueryOrder, Statement, Uuid, Value,
};
use error::{AppResult, Error};

use crate::data::{app_file::AppFile, query::Query as RequestQuery, response::Response};

use super::Repository;

pub(crate) struct Manage<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    owner_id: Uuid,
}

impl<'repository, T> Manage<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>, owner_id: Uuid) -> Self {
        Self {
            repository,
            owner_id,
        }
    }

    /// Alias to get the file metadata for the owner
    pub(crate) async fn file(&self, id: Uuid) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.owner_id).await?;

        if file.is_dir() {
            return Err(Error::BadRequest("file_not_found".to_string()));
        }

        Ok(file)
    }

    /// Find all files that are shared with the user
    pub(crate) async fn find(&self, request_query: RequestQuery) -> AppResult<Response> {
        let mut parents = vec![];

        let user_id = self.owner_id;
        let mut selector = self.repository.selector(user_id, true);

        if let Some(dir_id) = request_query.dir_id.as_ref() {
            let file_id = Uuid::from_str(dir_id)?;

            parents = self.dir_tree(file_id).await?;

            selector = selector.filter(files::Column::FileId.eq(file_id));
        } else {
            selector = selector.filter(files::Column::FileId.is_null());
        }

        let mut order = Order::Asc;
        if let Some(ord) = &request_query.order {
            if ord == "desc" {
                order = Order::Desc;
            }
        }

        if let Some(order_by) = request_query.order_by.as_ref() {
            let column = match order_by.as_str() {
                "created_at" => files::Column::FileCreatedAt,
                "size" => files::Column::Size,
                _ => return Err(Error::BadRequest("invalid_order_by".to_string())),
            };

            selector = selector.order_by(column, order);
        }

        selector
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await
            .map(|children| Response { parents, children })
            .map_err(Error::from)
    }

    /// Get the directory tree for the owner,
    /// tree is starting with the oldest parent leading all the way up to
    /// the given directory id
    pub(crate) async fn dir_tree(&self, id: Uuid) -> AppResult<Vec<AppFile>> {
        let sql = r#"
            WITH RECURSIVE file_tree(id, file_id) AS (
                SELECT id, file_id FROM files WHERE id = ? AND mime = 'dir'
                UNION ALL
                SELECT f.id, f.file_id FROM files f
                JOIN file_tree a ON a.file_id = f.id
            )
            SELECT * FROM file_tree;
        "#;

        let ids: Vec<Uuid> = files::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                self.repository.connection().get_database_backend(),
                sql,
                [id.into()],
            ))
            .into_json()
            .all(self.repository.connection())
            .await?
            .into_iter()
            .map(|json| {
                Uuid::from_str(json.get("id").unwrap().as_str().unwrap_or_default())
                    .unwrap_or_default()
            })
            .collect();

        let user_id = self.owner_id;

        let mut results = self
            .repository
            .selector(user_id, true)
            .filter(files::Column::Id.is_in(ids))
            .filter(files::Column::Mime.eq("dir"))
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await
            .map_err(Error::from)?;

        results.sort_by(|a, b| {
            if a.file_id.is_none() || a.file_id == Some(b.id) {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });

        if results.is_empty() {
            return Err(Error::NotFound("directory_not_found".to_string()));
        }

        Ok(results)
    }

    /// Get the file or a directory, if we get a directory we will also
    /// recursively get all the files and directories inside it
    pub(crate) async fn file_tree(&self, id: Uuid) -> AppResult<Vec<AppFile>> {
        let sql = r#"
            WITH RECURSIVE file_tree(id, file_id) AS (
            SELECT id, file_id FROM files WHERE id = ?
            UNION ALL
            SELECT child.id, child.file_id FROM files child
            JOIN file_tree parent ON parent.id = child.file_id
            )
            SELECT id, file_id FROM file_tree;
        "#;

        let ids = files::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                self.repository.connection().get_database_backend(),
                sql,
                [id.into()],
            ))
            .into_json()
            .all(self.repository.connection())
            .await?
            .into_iter()
            .map(|json| {
                let id = json.get("id").unwrap().as_str().unwrap_or_default();

                match Uuid::from_str(id) {
                    Ok(id) => id,
                    Err(_) => Uuid::nil(),
                }
            })
            .collect::<Vec<Uuid>>();

        let user_id = self.owner_id;

        let mut results = self
            .repository
            .selector(user_id, true)
            .filter(files::Column::Id.is_in(ids))
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await
            .map_err(Error::from)?;

        results.sort_by(|a, b| {
            if a.file_id.is_none() || a.file_id == Some(b.id) {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });

        if results.is_empty() {
            return Err(Error::NotFound("directory_not_found".to_string()));
        }

        Ok(results)
    }

    /// Load the file from the database by its name hash and by its parent id
    /// this method can be used to verify if you already have a file with the same name
    /// in the directory. In case the file already exist we can check if we could resume its upload
    pub(crate) async fn by_name<V>(&self, hash: V, parent_id: Option<i32>) -> AppResult<AppFile>
    where
        V: Into<Value> + Display + Clone,
    {
        let user_id = self.owner_id;

        let mut selector = self
            .repository
            .selector(user_id, true)
            .filter(files::Column::NameHash.eq(hash.clone()));

        if let Some(parent_id) = parent_id {
            selector = selector.filter(files::Column::FileId.eq(parent_id));
        } else {
            selector = selector.filter(files::Column::FileId.is_null());
        }

        selector
            .filter(user_files::Column::UserId.eq(user_id))
            .filter(user_files::Column::IsOwner.eq(true))
            .into_model::<AppFile>()
            .one(self.repository.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound("file_not_found".to_string()))
    }

    /// Delete many files or directories for the owner
    pub(crate) async fn delete_many(&self, ids: Vec<Uuid>) -> AppResult<u64> {
        let results = files::Entity::delete_many()
            .filter(files::Column::Id.is_in(ids))
            .exec(self.repository.connection())
            .await?;

        Ok(results.rows_affected)
    }

    /// Create a file entry in the database and set the owner with the
    /// sent encrypted_key.
    pub(crate) async fn create(
        &self,
        create_file: files::ActiveModel,
        encrypted_key: &str,
        hashed_tokens: Vec<String>,
    ) -> AppResult<AppFile> {
        // Check if the file_id is set, if it is, check if the parent is directory
        // and if the current user is the owner of that directory.
        if let Some(file_id) = create_file.file_id.clone().into_value() {
            if file_id.to_string().as_str() != "NULL" {
                let parent = self.repository.by_id(file_id, self.owner_id).await?;

                if !parent.is_owner || !parent.is_dir() {
                    return Err(Error::BadRequest("parent_directory_not_found".to_string()));
                }
            }
        }

        let file_id = entity::active_value_to_uuid(create_file.id.clone())
            .ok_or(Error::as_wrong_id("file"))?;

        files::Entity::insert(create_file)
            .exec_without_returning(self.repository.connection())
            .await?;

        self.repository
            .tokens(self.owner_id)
            .upsert(file_id, hashed_tokens)
            .await?;

        let id = entity::Uuid::new_v4();

        let user_file = user_files::ActiveModel {
            id: ActiveValue::Set(id),
            file_id: ActiveValue::Set(file_id),
            user_id: ActiveValue::Set(self.owner_id),
            is_owner: ActiveValue::Set(true),
            encrypted_key: ActiveValue::Set(encrypted_key.to_string()),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::NotSet,
        };

        user_files::Entity::insert(user_file)
            .exec_without_returning(self.repository.connection())
            .await?;

        self.repository
            .by_id(file_id, self.owner_id)
            .await
            .map(|f| f.is_new(true))
    }

    /// Finish the upload of a file by setting the finished_upload_at field
    pub(crate) async fn finish(&self, file: &AppFile) -> AppResult<AppFile> {
        if !file.is_owner || file.user_id != self.owner_id || file.is_dir() {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        let chunks = file
            .chunks
            .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

        files::ActiveModel {
            id: ActiveValue::Set(file.id),
            chunks_stored: ActiveValue::Set(Some(chunks)),
            finished_upload_at: ActiveValue::Set(Some(Utc::now().naive_utc())),
            ..Default::default()
        }
        .update(self.repository.connection())
        .await?;

        self.repository.by_id(file.id, file.user_id).await
    }
}
