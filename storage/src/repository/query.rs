//! Repository module for running query operations on files that will automatically filter
//! them for only the files where the user has the file shared with him.

use entity::{
    files, links, numeric::Numeric, user_files, ColumnTrait, Condition, ConnectionTrait,
    EntityTrait, Expr, IntoCondition, JoinType, QueryFilter, QuerySelect, RelationTrait, Uuid,
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

    /// Files whose bytes hash to `hash`, in any of the four digests stored at
    /// upload.
    ///
    /// Deliberately independent of the search index. "Do you already have
    /// these bytes" is a question about content, and answering it through the
    /// token index made it depend on whether a file happened to be indexed —
    /// so a file uploaded by a client that never wrote tags was invisible to a
    /// lookup that has nothing to do with tags.
    ///
    /// Access is still the `user_files` join, so a digest guessed or lifted
    /// from elsewhere reveals nothing the caller could not already list.
    pub(crate) async fn by_hash(&self, hash: &str, compact: bool) -> AppResult<Vec<AppFile>> {
        let selector = match compact {
            true => self.repository.compact_selector(self.user_id, false),
            false => self.repository.selector(self.user_id, false),
        };

        let results = selector
            .filter(
                Condition::any()
                    .add(files::Column::Md5.eq(hash))
                    .add(files::Column::Sha1.eq(hash))
                    .add(files::Column::Sha256.eq(hash))
                    .add(files::Column::Blake2b.eq(hash)),
            )
            // The selector left-joins `links`, so a file shared through more
            // than one link would otherwise come back once per link.
            .group_by(files::Column::Id)
            .group_by(user_files::Column::Id)
            .group_by(links::Column::Id)
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await?;

        Ok(results)
    }

    /// Sum all of the used space for the user so we can check if the user is over the quota limit
    pub(crate) async fn used_space(&self) -> AppResult<i64> {
        self.used_bytes_where(true).await
    }

    async fn used_bytes_where(&self, is_owner: bool) -> AppResult<i64> {
        let user_id = self.user_id;

        let bytes = user_files::Entity::find()
            .select_only()
            .filter(user_files::Column::UserId.eq(user_id))
            .join(
                JoinType::InnerJoin,
                user_files::Relation::Files
                    .def()
                    .on_condition(move |left, _right| {
                        Expr::col((left, user_files::Column::UserId))
                            .eq(user_id)
                            .and(user_files::Column::IsOwner.eq(is_owner))
                            .into_condition()
                    }),
            )
            .column_as(files::Column::Size.sum(), "sum_of_size")
            .group_by(user_files::Column::UserId)
            .into_tuple::<Option<Numeric>>()
            .one(self.repository.connection())
            .await?;

        Ok(bytes
            .unwrap_or_default()
            .map(|numeric| numeric.into())
            .unwrap_or(0))
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
