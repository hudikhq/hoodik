pub mod manage;
pub mod query;

use crate::data::app_file::AppFile;

use self::{manage::Manage, query::Query};
use entity::{
    files, user_files, users, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition,
    JoinType, QueryFilter, QuerySelect, RelationTrait, Uuid, Value,
};
use error::{AppResult, Error};
use std::fmt::Display;

pub struct Repository<'ctx, T: ConnectionTrait> {
    connection: &'ctx T,
}

impl<'ctx, T> Repository<'ctx, T>
where
    T: ConnectionTrait,
{
    pub fn new(connection: &'ctx T) -> Self {
        Self { connection }
    }
}

impl<'ctx, T> Repository<'ctx, T>
where
    T: ConnectionTrait,
{
    /// Query files from any user perspective
    pub fn query<'repository>(
        &'repository self,
        user: &'repository users::Model,
    ) -> Query<'repository, T>
    where
        Self: 'repository,
    {
        Query::<'repository>::new(self, user)
    }

    /// Manage files from the owners perspective
    pub fn manage<'repository>(
        &'repository self,
        owner: &'repository users::Model,
    ) -> Manage<'repository, T>
    where
        Self: 'repository,
    {
        Manage::<'repository>::new(self, owner)
    }

    /// Get the inner database connection
    pub fn connection(&self) -> &impl ConnectionTrait {
        self.connection
    }

    /// Load the file from the database by its id
    pub async fn by_id<V>(&self, id: V, user_id: Uuid) -> AppResult<AppFile>
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
