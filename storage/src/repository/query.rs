//! Repository module for running query operations on files that will automatically filter
//! them for only the files where the user has the file shared with him.

use entity::{
    files, user_files, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition, JoinType,
    QueryFilter, QuerySelect, RelationTrait, Uuid,
};
use error::AppResult;

use crate::data::{app_file::AppFile, stats::Stats};

use super::Repository;

pub(crate) struct Query<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    user_id: Uuid,
}

impl<'repository, T> Query<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>, user_id: Uuid) -> Self {
        Self {
            repository,
            user_id,
        }
    }

    /// Get any kind of file for the user
    pub(crate) async fn get(&self, id: Uuid) -> AppResult<AppFile> {
        let file = self.repository.by_id(id, self.user_id).await?;

        Ok(file)
    }

    /// Sum all of the used space for the user so we can check if the user is over the quota limit
    pub(crate) async fn used_space(&self) -> AppResult<i64> {
        let user_id = self.user_id;

        let used_space = user_files::Entity::find()
            .select_only()
            .filter(user_files::Column::UserId.eq(user_id))
            .join(
                JoinType::InnerJoin,
                user_files::Relation::Files
                    .def()
                    .on_condition(move |left, _right| {
                        Expr::col((left, user_files::Column::UserId))
                            .eq(user_id)
                            .and(user_files::Column::IsOwner.eq(true))
                            .into_condition()
                    }),
            )
            .column_as(files::Column::Size.sum(), "sum_of_size")
            .group_by(user_files::Column::UserId)
            .into_tuple::<Option<i64>>()
            .one(self.repository.connection())
            .await?;

        Ok(used_space.unwrap_or_default().unwrap_or(0))
    }

    /// Get the stats for the user about the used space and the quota
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
}
