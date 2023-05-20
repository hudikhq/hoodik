pub(crate) mod cached;
pub(crate) mod manage;
pub(crate) mod query;
pub(crate) mod tokens;

use crate::data::app_file::AppFile;

use self::{manage::Manage, query::Query, tokens::Tokens};
use entity::{
    files, user_files, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition, JoinType,
    QueryFilter, QuerySelect, RelationTrait, Uuid, Value,
};
use error::{AppResult, Error};
use std::fmt::Display;

pub(crate) struct Repository<'ctx, T: ConnectionTrait> {
    connection: &'ctx T,
}

impl<'ctx, T> Repository<'ctx, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(connection: &'ctx T) -> Self {
        Self { connection }
    }
}

impl<'ctx, T> Repository<'ctx, T>
where
    T: ConnectionTrait,
{
    /// Query files from any user perspective
    pub(crate) fn query<'repository>(&'repository self, user_id: Uuid) -> Query<'repository, T>
    where
        Self: 'repository,
    {
        Query::<'repository>::new(self, user_id)
    }

    /// Manage files from the owners perspective
    pub(crate) fn manage<'repository>(&'repository self, owner_id: Uuid) -> Manage<'repository, T>
    where
        Self: 'repository,
    {
        Manage::<'repository>::new(self, owner_id)
    }

    /// Manage files from the owners perspective
    pub(crate) fn tokens<'repository>(&'repository self, user_id: Uuid) -> Tokens<'repository, T>
    where
        Self: 'repository,
    {
        Tokens::<'repository>::new(self, user_id)
    }

    /// Get the inner database connection
    pub(crate) fn connection(&self) -> &impl ConnectionTrait {
        self.connection
    }

    /// Load the file from the database by its id
    pub(crate) async fn by_id<V>(&self, id: V, user_id: Uuid) -> AppResult<AppFile>
    where
        V: Into<Value> + Display + Clone,
    {
        let result = files::Entity::find()
            .filter(files::Column::Id.eq(id.clone()))
            .join(
                JoinType::InnerJoin,
                files::Relation::UserFiles
                    .def()
                    .on_condition(move |_left, right| {
                        Expr::col((right, user_files::Column::UserId))
                            .eq(user_id)
                            .into_condition()
                    }),
            )
            .select_also(user_files::Entity)
            .one(self.connection)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("file_not_found:{}", id)))?;

        let (file, user_file) = (result.0, result.1.unwrap());

        Ok(AppFile::from((file, user_file)))
    }
}
