use chrono::Utc;
use entity::{
    sort::Sortable, users, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, ModelTrait,
    QueryFilter, QuerySelect, Uuid,
};
use error::{AppResult, Error};
use validr::Validation;

use crate::data::users::search::Search;

use super::Repository;

pub(crate) struct UsersRepository<'ctx, T: ConnectionTrait> {
    repository: &'ctx Repository<'ctx, T>,
}

impl<'ctx, T> UsersRepository<'ctx, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'ctx Repository<'ctx, T>) -> Self {
        Self { repository }
    }

    /// Search through users
    pub(crate) async fn find(&self, users: Search) -> AppResult<Vec<entity::users::Model>> {
        let users = users.validate()?;

        let mut query = users::Entity::find();

        if let Some(sort) = users.sort {
            query = match users.order.as_deref() {
                Some("desc") => sort.sort_desc(query),
                _ => sort.sort_asc(query),
            };
        }

        if let Some(search) = users.search {
            let maybe_uuid = Uuid::parse_str(search.as_str()).ok();

            if let Some(uuid) = maybe_uuid {
                query = query.filter(users::Column::Id.eq(uuid));
            } else {
                query = query.filter(users::Column::Email.contains(search.as_str()));
            }
        }

        query = query.limit(users.limit.unwrap_or(15));
        query = query.offset(users.offset.unwrap_or(0));

        let users = query.all(self.repository.connection()).await?;

        Ok(users)
    }

    /// Find a single user by their id
    pub(crate) async fn get(&self, user_id: Uuid) -> AppResult<entity::users::Model> {
        let user = users::Entity::find_by_id(user_id)
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

        Ok(user)
    }

    /// Delete the user forever and all of their linked entities
    pub(crate) async fn delete(&self, user_id: Uuid) -> AppResult<()> {
        let user = users::Entity::find_by_id(user_id)
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

        let files = self.repository.files().find_for(user_id).await?;

        // We are deleting files specifically because they need
        // to run the purge on the fs as well, all other entities should
        // be automatically cascade deleted after the user is deleted.
        self.repository.files().delete_many(files).await?;

        user.delete(self.repository.connection()).await?;

        Ok(())
    }

    /// Delete the user forever and all of their linked entities
    pub(crate) async fn disable_tfa(&self, user_id: Uuid) -> AppResult<()> {
        let user = users::Entity::find_by_id(user_id)
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

        users::Entity::update(users::ActiveModel {
            id: ActiveValue::Set(user.id),
            secret: ActiveValue::Set(None),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            ..Default::default()
        })
        .exec(self.repository.connection())
        .await?;

        Ok(())
    }
}
