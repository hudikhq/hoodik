pub(crate) mod cached;
pub(crate) mod manage;
pub(crate) mod query;
pub(crate) mod tokens;

use crate::data::app_file::AppFile;

use self::{manage::Manage, query::Query, tokens::Tokens};
use entity::{
    files, links, user_files, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition,
    JoinType, QueryFilter, QuerySelect, RelationTrait, Select, Uuid, Value,
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

impl<T> Repository<'_, T>
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
        self.selector(user_id, false)
            .filter(files::Column::Id.eq(id.clone()))
            .into_model::<AppFile>()
            .one(self.connection)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("file_not_found:{id}")))
    }

    /// Preset the selector for the given user, maybe check if the user is the owner
    pub(crate) fn selector(&self, user_id: Uuid, check_is_owner: bool) -> Select<files::Entity> {
        let mut selector = files::Entity::find().select_only();

        entity::join::add_columns_with_prefix::<_, files::Entity>(&mut selector, "file");
        entity::join::add_columns_with_prefix::<_, user_files::Entity>(&mut selector, "user_file");
        entity::join::add_columns_with_prefix::<_, links::Entity>(&mut selector, "link");

        let rel = match check_is_owner {
            true => files::Relation::UserFiles
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, user_files::Column::UserId))
                        .eq(user_id)
                        .and(user_files::Column::IsOwner.eq(true))
                        .into_condition()
                }),
            false => files::Relation::UserFiles
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, user_files::Column::UserId))
                        .eq(user_id)
                        .into_condition()
                }),
        };

        selector.join(JoinType::InnerJoin, rel).join(
            JoinType::LeftJoin,
            files::Relation::Links
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, links::Column::UserId))
                        .eq(user_id)
                        .into_condition()
                }),
        )
    }
}
