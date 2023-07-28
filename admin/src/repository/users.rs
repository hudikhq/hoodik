use chrono::Utc;
use entity::{
    paginated::Paginated, sessions, sort::Sortable, users, ActiveValue, ColumnTrait,
    ConnectionTrait, EntityTrait, Expr, IntoCondition, JoinType, ModelTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select, Uuid,
};
use error::{AppResult, Error};
use validr::Validation;

use crate::data::users::{search::Search, update::Update, user::User};

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

    /// Query builder that creates a join query for the user and session
    fn join_query(&self) -> Select<users::Entity> {
        let mut query = users::Entity::find().select_only();

        entity::join::add_columns_with_prefix::<_, users::Entity>(&mut query, "user");
        entity::join::add_columns_with_prefix::<_, sessions::Entity>(&mut query, "session");

        query = query.join(
            JoinType::LeftJoin,
            users::Relation::Sessions
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, sessions::Column::ExpiresAt))
                        .gt(Utc::now().timestamp())
                        .and(sessions::Column::Refresh.is_not_null())
                        .into_condition()
                }),
        );

        query = query.order_by_desc(sessions::Column::UpdatedAt);
        query = query.group_by(users::Column::Id);

        query
    }

    /// Search through users
    pub(crate) async fn find(&self, users: Search) -> AppResult<Paginated<User>> {
        let users = users.validate()?;

        let mut query = self.join_query();

        let sort = users.sort.unwrap_or_default();

        query = match users.order.as_deref() {
            Some("desc") => sort.sort_desc(query),
            _ => sort.sort_asc(query),
        };

        if let Some(search) = users.search {
            let maybe_uuid = Uuid::parse_str(search.as_str()).ok();

            if let Some(uuid) = maybe_uuid {
                query = query.filter(users::Column::Id.eq(uuid));
            } else {
                query = query.filter(users::Column::Email.contains(search.as_str()));
            }
        }

        let total = query.clone().count(self.repository.connection()).await?;

        query = query.limit(users.limit.unwrap_or(15));
        query = query.offset(users.offset.unwrap_or(0));

        let users = query
            .into_model::<User>()
            .all(self.repository.connection())
            .await?;

        Ok(Paginated::new(users, total))
    }

    /// Find a single user by their id
    pub(crate) async fn get(&self, user_id: Uuid) -> AppResult<User> {
        let user = self
            .join_query()
            .filter(users::Column::Id.eq(user_id))
            .into_model::<User>()
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

        Ok(user)
    }

    /// Update user information
    pub(crate) async fn update(&self, user_id: Uuid, update: Update) -> AppResult<User> {
        let update = update.validate()?;

        let user = self
            .join_query()
            .filter(users::Column::Id.eq(user_id))
            .into_model::<User>()
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| Error::NotFound("User not found".to_string()))?;

        let active_model = users::ActiveModel {
            id: ActiveValue::Set(user.id),
            role: ActiveValue::Set(update.role),
            quota: ActiveValue::Set(update.quota),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            ..Default::default()
        };

        users::Entity::update(active_model)
            .exec(self.repository.connection())
            .await?;

        self.get(user_id).await
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
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            ..Default::default()
        })
        .exec(self.repository.connection())
        .await?;

        Ok(())
    }
}
