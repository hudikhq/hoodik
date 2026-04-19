//! Repository module for manipulating with files in the database
//! this module should only be used by the owner of the file
use std::{collections::HashMap, fmt::Display, str::FromStr};

use chrono::Utc;
use entity::{
    file_versions, files, user_files, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait,
    EntityTrait, Order, QueryFilter, QueryOrder, Statement, Uuid, Value,
};
use error::{AppResult, Error};
use validr::Validation;

use super::Repository;
use crate::data::{
    app_file::AppFile, query::Query as RequestQuery, rename::Rename,
    replace_content::ValidatedReplaceContent, response::Response, set_editable::SetEditable,
    update_hashes::UpdateHashes,
};
use futures::future::try_join_all;

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

    /// Find all files and folders that are shared with the user
    pub(crate) async fn find(&self, request_query: RequestQuery) -> AppResult<Response> {
        let mut parents = vec![];

        let user_id = self.owner_id;
        let mut selector = self
            .repository
            .selector(user_id, true)
            .filter(user_files::Column::IsOwner.eq(request_query.is_owner.unwrap_or(true)));

        if let Some(dir_id) = request_query.dir_id.as_ref() {
            let file_id = Uuid::from_str(dir_id)?;

            parents = self.dir_tree(file_id).await?;

            selector = selector.filter(files::Column::FileId.eq(file_id));
        } else if request_query.editable.is_some() {
            // When filtering by editable, return files from ALL folders
        } else {
            selector = selector.filter(files::Column::FileId.is_null());
        }

        if request_query.dirs_only.unwrap_or(false) {
            selector = selector.filter(files::Column::Mime.eq("dir"));
        }

        if let Some(editable) = request_query.editable {
            selector = selector.filter(files::Column::Editable.eq(editable));
        }

        let mut order = Order::Asc;
        if let Some(ord) = &request_query.order {
            if ord == "desc" {
                order = Order::Desc;
            }
        }

        if let Some(order_by) = request_query.order_by.as_ref() {
            let column = match order_by.as_str() {
                "modified_at" => files::Column::FileModifiedAt,
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
                SELECT id, file_id FROM files WHERE id = $1 AND mime = 'dir'
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

        // Sort by ascending depth so ancestors appear in root-first (breadcrumb) order.
        let id_to_idx: HashMap<Uuid, usize> =
            results.iter().enumerate().map(|(i, f)| (f.id, i)).collect();
        let depths: Vec<usize> = (0..results.len())
            .map(|i| {
                let mut depth = 0usize;
                let mut current = results[i].file_id;
                while let Some(parent_id) = current {
                    if let Some(&parent_idx) = id_to_idx.get(&parent_id) {
                        depth += 1;
                        current = results[parent_idx].file_id;
                    } else {
                        break;
                    }
                }
                depth
            })
            .collect();
        results.sort_by(|a, b| depths[id_to_idx[&a.id]].cmp(&depths[id_to_idx[&b.id]]));

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
            SELECT id, file_id FROM files WHERE id = $1
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

        let results = self
            .repository
            .selector(user_id, true)
            .filter(files::Column::Id.is_in(ids))
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await
            .map_err(Error::from)?;

        if results.is_empty() {
            return Err(Error::NotFound("directory_not_found".to_string()));
        }

        Ok(results)
    }

    /// Load the file from the database by its name hash and by its parent id
    /// this method can be used to verify if you already have a file with the same name
    /// in the directory. In case the file already exist we can check if we could resume its upload
    pub(crate) async fn by_name<V>(&self, hash: V, parent_id: Option<Uuid>) -> AppResult<AppFile>
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

    /// Move multiple files and folders to a new parent directory
    pub(crate) async fn move_many(
        &self,
        mut ids: Vec<Uuid>,
        file_id: Option<Uuid>,
    ) -> AppResult<u64> {
        ids.sort();
        ids.dedup();

        if let Some(f) = file_id {
            if ids.contains(&f) {
                return Err(Error::BadRequest("cannot_move_to_itself".to_string()));
            }
        }

        let existing_file_ids = self
            .repository
            .selector(self.owner_id, true)
            .filter(files::Column::Id.is_in(ids))
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await?
            .into_iter()
            .map(|f| f.id)
            .collect::<Vec<_>>();

        let active_model = files::ActiveModel {
            file_id: ActiveValue::Set(file_id),
            ..Default::default()
        };

        let results = files::Entity::update_many()
            .filter(files::Column::Id.is_in(existing_file_ids))
            .set(active_model)
            .exec(self.repository.connection())
            .await?;

        Ok(results.rows_affected)
    }

    /// Rename a file or directory for the owner
    pub(crate) async fn rename(&self, id: Uuid, data: Rename) -> AppResult<AppFile> {
        let (active_model, hashed_tokens, name_hash) = data.into_active_model(id)?;

        let file = self.repository.by_id(id, self.owner_id).await?;

        if self.by_name(&name_hash, file.file_id).await.is_ok() {
            return Err(Error::BadRequest("file_already_exists".to_string()));
        }

        if !file.is_owner || file.user_id != self.owner_id {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        active_model.update(self.repository.connection()).await?;

        self.repository
            .tokens(self.owner_id)
            .rename(id, hashed_tokens)
            .await?;

        self.repository.by_id(file.id, file.user_id).await
    }

    /// Delete many files or directories for the owner
    pub(crate) async fn delete_many(&self, ids: Vec<Uuid>) -> AppResult<Vec<AppFile>> {
        let mut files = try_join_all(ids.into_iter().map(|id| self.file_tree(id)))
            .await?
            .into_iter()
            .flatten()
            .filter(|f| f.is_owner)
            .collect::<Vec<AppFile>>();

        // Sort files by id and then run dedup_by to remove all duplicates,
        // without sorting, this won't remove all duplicates
        files.sort_by(|a, b| a.id.cmp(&b.id));
        files.dedup_by(|a, b| a.id == b.id);

        let ids: Vec<Uuid> = files.iter().map(|f| f.id).collect();

        files::Entity::delete_many()
            .filter(files::Column::Id.is_in(ids))
            .exec(self.repository.connection())
            .await?;

        Ok(files)
    }

    /// Create a file entry in the database and set the owner with the
    /// sent encrypted_key.
    pub(crate) async fn create(
        &self,
        create_file: files::ActiveModel,
        encrypted_key: &str,
        hashed_tokens: Vec<String>,
    ) -> AppResult<AppFile> {
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
            created_at: ActiveValue::Set(Utc::now().timestamp()),
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

    /// Update the content hashes of a file after upload completes
    pub(crate) async fn update_hashes(
        &self,
        id: Uuid,
        data: UpdateHashes,
    ) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.owner_id).await?;

        if !file.is_owner || file.user_id != self.owner_id || file.is_dir() {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        let active_model = data.into_active_model(id)?;
        active_model.update(self.repository.connection()).await?;

        self.repository.by_id(file.id, file.user_id).await
    }

    /// Allocate a pending version and stage the in-flight upload metadata.
    ///
    /// **Does not touch chunks on disk** — the active version stays fully
    /// readable for the duration of the edit. Once `chunks_stored ==
    /// pending_chunks` the upload route fires [`Self::finish`], which
    /// atomically swaps `active_version = pending_version`.
    ///
    /// Returns `(file, abandoned_pending)`:
    /// - `file` is the post-update record for the response.
    /// - `abandoned_pending` is `Some(N)` only when `force = true` was used
    ///   to bypass an in-flight pending — the caller is responsible for
    ///   purging that version's chunk directory after the DB commit.
    ///
    /// Errors:
    /// - `Conflict("another_edit_is_in_progress")` if a pending exists and
    ///   `force = false`. Surfaces as HTTP 409.
    /// - `BadRequest("cannot_replace_directory")` for directories.
    /// - `BadRequest("file_not_editable")` when the file is not flagged
    ///   `editable`.
    pub(crate) async fn replace_content(
        &self,
        id: Uuid,
        data: ValidatedReplaceContent,
    ) -> AppResult<(AppFile, Option<i32>)> {
        let file = self.repository.by_id(id, self.owner_id).await?;

        if !file.is_owner || file.user_id != self.owner_id {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        if file.is_dir() {
            return Err(Error::BadRequest("cannot_replace_directory".to_string()));
        }

        if !file.editable {
            return Err(Error::BadRequest("file_not_editable".to_string()));
        }

        // Concurrent-edit guard. The client opts into recovery via
        // `force = true`, abandoning the previous pending edit.
        let abandoned_pending = if let Some(pending) = file.pending_version {
            if !data.force {
                return Err(Error::Conflict(
                    "another_edit_is_in_progress".to_string(),
                ));
            }
            Some(pending)
        } else {
            None
        };

        // Allocate the next pending version. With `force`, take a slot
        // strictly above any abandoned pending so straggler chunk uploads
        // from the dying client can never accidentally land in the new
        // pending dir.
        let next = std::cmp::max(file.active_version, abandoned_pending.unwrap_or(0)) + 1;
        let now = Utc::now().timestamp();

        let mut active_model = files::ActiveModel {
            id: ActiveValue::Set(id),
            pending_version: ActiveValue::Set(Some(next)),
            pending_chunks: ActiveValue::Set(Some(data.chunks)),
            pending_size: ActiveValue::Set(Some(data.size)),
            chunks_stored: ActiveValue::Set(Some(0)),
            file_modified_at: ActiveValue::Set(now),
            ..Default::default()
        };

        // Metadata-only updates swap in immediately — they don't affect
        // chunk decryption, so concurrent readers seeing the new name with
        // the old content for a few seconds is harmless.
        if let Some(name) = data.encrypted_name {
            active_model.encrypted_name = ActiveValue::Set(name);
        }
        if let Some(thumbnail) = data.encrypted_thumbnail {
            active_model.encrypted_thumbnail = ActiveValue::Set(Some(thumbnail));
        }

        active_model.update(self.repository.connection()).await?;

        self.repository
            .tokens(self.owner_id)
            .rename(id, data.search_tokens_hashed)
            .await?;

        let file = self.repository.by_id(id, self.owner_id).await?;
        Ok((file, abandoned_pending))
    }

    /// Toggle the `editable` flag on an existing file.
    /// Only the owner can convert a regular file into an editable note (or back).
    /// Directories are rejected — `editable` is a file-level concept.
    ///
    /// The flag drives chunk-layout routing (see `AppFile::use_versioned_layout`),
    /// so a flip mid-edit or after history accumulates would orphan chunks.
    /// Two guards block that:
    ///
    /// - Flipping while `pending_version.is_some()` — rejected, because
    ///   the in-flight edit is targeting the versioned path; switching
    ///   mid-stream would strand its chunks.
    /// - Flipping `editable → false` while history exists (either
    ///   `active_version > 1` or any `file_versions` row) — rejected,
    ///   because the versioned directories would become unreachable
    ///   through the legacy read path. Deleting the file is the only way
    ///   to go back.
    ///
    /// Going `non-editable → editable` on a file with only legacy chunks
    /// is allowed with no disk I/O here. The first subsequent edit snapshots
    /// the legacy chunks as v1 and lands the new content in v2 via the
    /// existing `copy_version` fallback.
    pub(crate) async fn set_editable(&self, id: Uuid, data: SetEditable) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.owner_id).await?;

        if !file.is_owner || file.user_id != self.owner_id {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        if file.is_dir() {
            return Err(Error::BadRequest("cannot_set_editable_on_directory".to_string()));
        }

        let validated = data.validate()?;
        let next_editable = validated.editable.unwrap();

        if next_editable != file.editable && file.has_pending_upload() {
            return Err(Error::Conflict(
                "cannot_change_editable_during_edit".to_string(),
            ));
        }

        if file.editable && !next_editable {
            let has_history = file.active_version > 1
                || file_versions::Entity::find()
                    .filter(file_versions::Column::FileId.eq(id))
                    .one(self.repository.connection())
                    .await?
                    .is_some();
            if has_history {
                return Err(Error::Conflict(
                    "cannot_disable_editable_with_history".to_string(),
                ));
            }
        }

        files::ActiveModel {
            id: ActiveValue::Set(id),
            editable: ActiveValue::Set(next_editable),
            ..Default::default()
        }
        .update(self.repository.connection())
        .await?;

        self.repository.by_id(id, self.owner_id).await
    }

    /// Commit either an initial upload or an edit. Auto-fired by the
    /// upload route once `chunks_stored` matches the target chunk count.
    ///
    /// **Initial upload** (`pending_version` is None): just sets
    /// `finished_upload_at`. No `file_versions` row is created — the
    /// first-ever content has no historical predecessor to snapshot.
    ///
    /// **Edit** (`pending_version` is Some): inserts a `file_versions`
    /// row for the outgoing active version, then atomically swaps the
    /// pointer. The `(file_id, version)` unique index makes the snapshot
    /// idempotent — a retried last chunk that double-fires `finish`
    /// errors out cleanly instead of flipping pointers twice.
    ///
    /// After the swap, prunes history over `max_file_versions` and
    /// returns the version numbers that were dropped from the database.
    /// The caller is expected to wipe their on-disk directories
    /// (`fs.purge_version`) post-commit; failures there are best-effort.
    ///
    /// Caller is expected to wrap this in a DB transaction so snapshot,
    /// swap, and prune commit together — the upload route does that.
    pub(crate) async fn finish(
        &self,
        file: &AppFile,
        max_file_versions: usize,
    ) -> AppResult<(AppFile, Vec<i32>)> {
        if !file.is_owner || file.user_id != self.owner_id || file.is_dir() {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        let now = Utc::now().timestamp();
        let conn = self.repository.connection();

        if let Some(pending) = file.pending_version {
            // Snapshot the outgoing active version into history.
            let snapshot = file_versions::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                file_id: ActiveValue::Set(file.id),
                version: ActiveValue::Set(file.active_version),
                user_id: ActiveValue::Set(Some(self.owner_id)),
                is_anonymous: ActiveValue::Set(false),
                size: ActiveValue::Set(file.size.unwrap_or(0)),
                chunks: ActiveValue::Set(file.chunks.unwrap_or(0)),
                sha256: ActiveValue::Set(file.sha256.clone()),
                created_at: ActiveValue::Set(now),
            };
            file_versions::Entity::insert(snapshot)
                .exec_without_returning(conn)
                .await?;

            let new_chunks = file
                .pending_chunks
                .ok_or_else(|| Error::BadRequest("pending_chunks_not_set".to_string()))?;
            let new_size = file
                .pending_size
                .ok_or_else(|| Error::BadRequest("pending_size_not_set".to_string()))?;

            // Atomic pointer swap. Hashes go to NULL — `update_hashes`
            // immediately after this fills them for the new active version.
            files::ActiveModel {
                id: ActiveValue::Set(file.id),
                active_version: ActiveValue::Set(pending),
                pending_version: ActiveValue::Set(None),
                pending_chunks: ActiveValue::Set(None),
                pending_size: ActiveValue::Set(None),
                chunks: ActiveValue::Set(Some(new_chunks)),
                size: ActiveValue::Set(Some(new_size)),
                chunks_stored: ActiveValue::Set(Some(new_chunks)),
                md5: ActiveValue::Set(None),
                sha1: ActiveValue::Set(None),
                sha256: ActiveValue::Set(None),
                blake2b: ActiveValue::Set(None),
                finished_upload_at: ActiveValue::Set(Some(now)),
                ..Default::default()
            }
            .update(conn)
            .await?;
        } else {
            // Brand-new file's first commit. No swap, no history row.
            let chunks = file
                .chunks
                .ok_or_else(|| Error::BadRequest("file_has_no_chunks".to_string()))?;

            files::ActiveModel {
                id: ActiveValue::Set(file.id),
                chunks_stored: ActiveValue::Set(Some(chunks)),
                finished_upload_at: ActiveValue::Set(Some(now)),
                ..Default::default()
            }
            .update(conn)
            .await?;
        }

        // Prune oldest history beyond the cap. Inside the same transaction
        // so a partial prune can never escape — either we drop these rows
        // and the swap commits, or both roll back.
        let pruned = self
            .repository
            .versions(self.owner_id)
            .prune_over_cap(file.id, max_file_versions)
            .await?;

        let updated = self.repository.by_id(file.id, file.user_id).await?;
        Ok((updated, pruned))
    }
}
