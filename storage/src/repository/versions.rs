//! Version-history operations on a single file.
//!
//! Built on the versioned-chunks layout where every committed version
//! lives at `{file_id}/v{N}/`. All operations here are "pointer flips
//! plus optional chunk copies" — the storage layer doesn't decrypt or
//! re-encrypt anything.
//!
//! Restore semantics: restoring an old version doesn't *demote* the
//! current active version — it's preserved in history. Restore allocates
//! a fresh version slot, copies chunks from the target into it, and
//! flips `active_version` to point at the new copy. This way a user can
//! restore v1, change their mind, and restore v3 — and v3 is still on
//! disk because it was never deleted.

use chrono::Utc;
use entity::{
    file_versions, files, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait,
    Order, QueryFilter, QueryOrder, Uuid,
};
use error::{AppResult, Error};

use super::Repository;

pub(crate) struct Versions<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    owner_id: Uuid,
}

/// Outcome of a successful restore-in-place. The route consumes this to
/// (a) copy chunks on disk into the new version slot and (b) optionally
/// purge any pruned versions.
pub(crate) struct RestoreOutcome {
    /// Source version whose chunks should be duplicated into `new_version`.
    pub source_version: i32,
    /// Destination version slot the active pointer now references.
    pub new_version: i32,
}

/// Outcome of a successful fork. The new file shares the source's chunk
/// payload (same encryption key, copy-on-write) but is otherwise an
/// independent record with its own history.
pub(crate) struct ForkOutcome {
    pub new_file_id: Uuid,
    pub source_version: i32,
}

impl<'repository, T> Versions<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>, owner_id: Uuid) -> Self {
        Self {
            repository,
            owner_id,
        }
    }

    /// List historical versions for a file, newest first. Excludes the
    /// currently-active version (it's on the file row, not in history).
    pub(crate) async fn list(&self, file_id: Uuid) -> AppResult<Vec<file_versions::Model>> {
        // Owner-only access is enforced by walking through `by_id` first;
        // it 404s anyone who isn't the owner.
        let _ = self.repository.by_id(file_id, self.owner_id).await?;

        file_versions::Entity::find()
            .filter(file_versions::Column::FileId.eq(file_id))
            .order_by(file_versions::Column::Version, Order::Desc)
            .all(self.repository.connection())
            .await
            .map_err(Error::from)
    }

    /// Look up a specific version's metadata. Used by the download route
    /// for `GET /versions/{version}?format=tar`.
    pub(crate) async fn get(
        &self,
        file_id: Uuid,
        version: i32,
    ) -> AppResult<file_versions::Model> {
        let _ = self.repository.by_id(file_id, self.owner_id).await?;

        file_versions::Entity::find()
            .filter(file_versions::Column::FileId.eq(file_id))
            .filter(file_versions::Column::Version.eq(version))
            .one(self.repository.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound("version_not_found".to_string()))
    }

    /// Restore an old version into the file's active slot.
    ///
    /// Allocates a brand-new version (above any existing or pending one),
    /// signals to the route which on-disk source to copy from, and flips
    /// the active pointer once the route has duplicated the chunks. The
    /// previously-active version is snapshotted into history along the
    /// way, so nothing is lost.
    pub(crate) async fn restore(
        &self,
        file_id: Uuid,
        target_version: i32,
    ) -> AppResult<RestoreOutcome> {
        let file = self.repository.by_id(file_id, self.owner_id).await?;

        if !file.is_owner || file.user_id != self.owner_id || file.is_dir() {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        // Reject if an edit is in flight — restore + concurrent edit would
        // race for the same `active_version` slot. The user can use the
        // existing 409-with-force flow on the in-flight edit if they
        // really mean to abandon it.
        if file.has_pending_upload() {
            return Err(Error::Conflict(
                "another_edit_is_in_progress".to_string(),
            ));
        }

        // Self-restore is a no-op — bail before allocating a slot.
        if file.active_version == target_version {
            return Err(Error::BadRequest(
                "version_already_active".to_string(),
            ));
        }

        let target = self.get(file_id, target_version).await?;

        // Bump above the current max version on either side.
        let max_existing = file_versions::Entity::find()
            .filter(file_versions::Column::FileId.eq(file_id))
            .order_by(file_versions::Column::Version, Order::Desc)
            .one(self.repository.connection())
            .await?
            .map(|v| v.version)
            .unwrap_or(file.active_version);
        let next = std::cmp::max(file.active_version, max_existing) + 1;

        let now = Utc::now().timestamp();

        // Snapshot the outgoing active into history before flipping.
        let snapshot = file_versions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            file_id: ActiveValue::Set(file_id),
            version: ActiveValue::Set(file.active_version),
            user_id: ActiveValue::Set(Some(self.owner_id)),
            is_anonymous: ActiveValue::Set(false),
            size: ActiveValue::Set(file.size.unwrap_or(0)),
            chunks: ActiveValue::Set(file.chunks.unwrap_or(0)),
            sha256: ActiveValue::Set(file.sha256.clone()),
            created_at: ActiveValue::Set(now),
        };
        file_versions::Entity::insert(snapshot)
            .exec_without_returning(self.repository.connection())
            .await?;

        // Flip pointers. `chunks/size/sha256` come from the target's
        // recorded metadata so the restored content's hashes match
        // exactly — no need for a follow-up `update_hashes` call.
        files::ActiveModel {
            id: ActiveValue::Set(file_id),
            active_version: ActiveValue::Set(next),
            chunks: ActiveValue::Set(Some(target.chunks)),
            size: ActiveValue::Set(Some(target.size)),
            chunks_stored: ActiveValue::Set(Some(target.chunks)),
            sha256: ActiveValue::Set(target.sha256.clone()),
            md5: ActiveValue::Set(None),
            sha1: ActiveValue::Set(None),
            blake2b: ActiveValue::Set(None),
            finished_upload_at: ActiveValue::Set(Some(now)),
            file_modified_at: ActiveValue::Set(now),
            ..Default::default()
        }
        .update(self.repository.connection())
        .await?;

        Ok(RestoreOutcome {
            source_version: target_version,
            new_version: next,
        })
    }

    /// Restore a version into a brand-new file, leaving the original
    /// untouched. The caller (route) constructs the new file's name and
    /// metadata client-side — same shape as `createFile` — and passes the
    /// pre-built `files` ActiveModel + encrypted_key here. This module
    /// handles the database wiring and chunk copy bookkeeping.
    pub(crate) async fn fork(
        &self,
        source_file_id: Uuid,
        source_version: i32,
        new_file_active_model: files::ActiveModel,
        encrypted_key: String,
    ) -> AppResult<ForkOutcome> {
        let source = self.repository.by_id(source_file_id, self.owner_id).await?;

        if !source.is_owner || source.user_id != self.owner_id {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        // Verify the source version exists (or is the active version,
        // which doesn't appear in `file_versions`).
        if source.active_version != source_version {
            self.get(source_file_id, source_version).await?;
        }

        // Insert the new file row + ownership record. Mirrors the
        // create-file flow but skips the search-token plumbing — fork is
        // primarily a recovery feature, the user can rename or re-tokenize
        // the new note later.
        let new_file_id = entity::active_value_to_uuid(new_file_active_model.id.clone())
            .ok_or(Error::as_wrong_id("file"))?;

        files::Entity::insert(new_file_active_model)
            .exec_without_returning(self.repository.connection())
            .await?;

        let user_file = entity::user_files::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            file_id: ActiveValue::Set(new_file_id),
            user_id: ActiveValue::Set(self.owner_id),
            is_owner: ActiveValue::Set(true),
            encrypted_key: ActiveValue::Set(encrypted_key),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            expires_at: ActiveValue::NotSet,
        };
        entity::user_files::Entity::insert(user_file)
            .exec_without_returning(self.repository.connection())
            .await?;

        Ok(ForkOutcome {
            new_file_id,
            source_version,
        })
    }

    /// Delete a single historical version. The active version cannot be
    /// deleted this way — use the file deletion path for that.
    pub(crate) async fn delete(&self, file_id: Uuid, version: i32) -> AppResult<()> {
        let file = self.repository.by_id(file_id, self.owner_id).await?;

        if !file.is_owner || file.user_id != self.owner_id {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        if file.active_version == version {
            return Err(Error::BadRequest(
                "cannot_delete_active_version".to_string(),
            ));
        }

        // Confirm the row exists before delete so we can return a clear
        // 404 instead of "0 rows affected".
        let _ = self.get(file_id, version).await?;

        file_versions::Entity::delete_many()
            .filter(file_versions::Column::FileId.eq(file_id))
            .filter(file_versions::Column::Version.eq(version))
            .exec(self.repository.connection())
            .await?;

        Ok(())
    }

    /// Purge every historical version for a file, keeping only the
    /// current active. The route follows up by deleting the corresponding
    /// `v{N}/` directories on disk.
    pub(crate) async fn purge_all_history(&self, file_id: Uuid) -> AppResult<Vec<i32>> {
        let file = self.repository.by_id(file_id, self.owner_id).await?;

        if !file.is_owner || file.user_id != self.owner_id {
            return Err(Error::NotFound("file_not_found".to_string()));
        }

        let rows = file_versions::Entity::find()
            .filter(file_versions::Column::FileId.eq(file_id))
            .all(self.repository.connection())
            .await?;
        let versions: Vec<i32> = rows.iter().map(|v| v.version).collect();

        file_versions::Entity::delete_many()
            .filter(file_versions::Column::FileId.eq(file_id))
            .exec(self.repository.connection())
            .await?;

        Ok(versions)
    }

    /// Drop the oldest historical versions when the count exceeds `cap`.
    /// Returns the version numbers that were pruned so the route can
    /// remove their on-disk directories.
    ///
    /// Best-effort by design: we prune AFTER a successful finalize, and a
    /// failure here doesn't roll back the commit. The next successful
    /// commit will catch up.
    pub(crate) async fn prune_over_cap(
        &self,
        file_id: Uuid,
        cap: usize,
    ) -> AppResult<Vec<i32>> {
        let mut rows = file_versions::Entity::find()
            .filter(file_versions::Column::FileId.eq(file_id))
            .order_by(file_versions::Column::Version, Order::Asc)
            .all(self.repository.connection())
            .await?;

        if rows.len() <= cap {
            return Ok(vec![]);
        }

        let to_drop = rows.drain(..rows.len() - cap).collect::<Vec<_>>();
        let drop_ids: Vec<Uuid> = to_drop.iter().map(|v| v.id).collect();
        let drop_versions: Vec<i32> = to_drop.iter().map(|v| v.version).collect();

        file_versions::Entity::delete_many()
            .filter(file_versions::Column::Id.is_in(drop_ids))
            .exec(self.repository.connection())
            .await?;

        Ok(drop_versions)
    }
}
