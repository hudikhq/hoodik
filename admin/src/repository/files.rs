use crate::data::files::stats::Stats;

use super::Repository;
use entity::{
    files, user_files, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition, JoinType,
    QueryFilter, QuerySelect, RelationTrait, Uuid,
};
use error::AppResult;
use fs::prelude::*;

pub(crate) struct FilesRepository<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
}

impl<'repository, T> FilesRepository<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>) -> Self {
        Self { repository }
    }

    /// Get global file stats
    pub(crate) async fn stats(&self) -> AppResult<Vec<Stats>> {
        let stats = files::Entity::find()
            .select_only()
            .filter(files::Column::Mime.ne("dir"))
            .column_as(files::Column::Mime, "mime")
            .column_as(files::Column::Size.sum(), "size")
            .column_as(files::Column::Id.count(), "count")
            .group_by(files::Column::Mime)
            .into_model::<Stats>()
            .all(self.repository.connection())
            .await?;

        Ok(stats)
    }

    /// Get file stats for a single user
    pub(crate) async fn stats_for(&self, user_id: Uuid) -> AppResult<Vec<Stats>> {
        let stats = user_files::Entity::find()
            .select_only()
            .filter(user_files::Column::UserId.eq(user_id))
            .filter(user_files::Column::IsOwner.eq(true))
            .join(
                JoinType::InnerJoin,
                user_files::Relation::Files
                    .def()
                    .on_condition(move |_left, right| {
                        Expr::col((right, files::Column::Mime))
                            .ne("dir")
                            .into_condition()
                    }),
            )
            .column_as(files::Column::Mime, "mime")
            .column_as(files::Column::Size.sum(), "size")
            .column_as(files::Column::Id.count(), "count")
            .group_by(files::Column::Mime)
            .into_model::<Stats>()
            .all(self.repository.connection())
            .await?;

        Ok(stats)
    }

    /// Find all the files from a single user, this will make sure to
    /// delete them all when the user is deleted.
    pub(crate) async fn find_for(&self, user_id: Uuid) -> AppResult<Vec<entity::files::Model>> {
        let files = user_files::Entity::find()
            .filter(user_files::Column::UserId.eq(user_id))
            .filter(user_files::Column::IsOwner.eq(true))
            .join(JoinType::InnerJoin, user_files::Relation::Files.def())
            .select_also(files::Entity)
            .all(self.repository.connection())
            .await?
            .into_iter()
            .map(|(_, file)| file.unwrap())
            .collect::<Vec<files::Model>>();

        Ok(files)
    }

    /// Delete many files at once, remove the storage data as well
    pub(crate) async fn delete_many(&self, files: Vec<files::Model>) -> AppResult<u64> {
        let fs = Fs::new(&self.repository.context().config);

        for file in files.iter() {
            if file.mime.as_str() != "dir" {
                // Drop every version + legacy chunks; safe across both layouts.
                fs.purge_all(file).await?;
            }
        }

        Ok(files::Entity::delete_many()
            .filter(files::Column::Id.is_in(files.into_iter().map(|f| f.id).collect::<Vec<_>>()))
            .exec(self.repository.connection())
            .await?
            .rows_affected)
    }

    /// Get the available space on the storage provider. When an instance-wide
    /// quota is configured, that quota is the real ceiling — an S3-backed
    /// provider otherwise reports `u64::MAX` (no such thing as "disk free space"
    /// on object storage), which is meaningless to show an operator.
    pub(crate) async fn available_space(&self, used: i64) -> AppResult<u64> {
        if let Some(quota) = self.repository.context().config.app.storage_instance_quota_bytes {
            return Ok(quota.saturating_sub(used.max(0) as u64));
        }

        let fs = Fs::new(&self.repository.context().config);
        fs.available_space().await
    }
}
