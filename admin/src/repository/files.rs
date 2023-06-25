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

    /// Get file stats for a single user
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

    /// Delete many files at once
    pub(crate) async fn delete_many(&self, files: Vec<files::Model>) -> AppResult<u64> {
        let fs = Fs::new(&self.repository.context().config);

        for file in files.iter() {
            if file.mime.as_str() != "dir" {
                fs.purge(file).await?;
            }
        }

        Ok(files::Entity::delete_many()
            .filter(files::Column::Id.is_in(files.into_iter().map(|f| f.id).collect::<Vec<_>>()))
            .exec(self.repository.connection())
            .await?
            .rows_affected)
    }

    /// Get the available space on the storage provider
    pub(crate) async fn available_space(&self) -> AppResult<u64> {
        let fs = Fs::new(&self.repository.context().config);
        fs.available_space().await
    }
}
