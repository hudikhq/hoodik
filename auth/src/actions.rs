use entity::{user_actions, users, ConnectionTrait, EntityTrait};
use error::{AppResult, Error};

pub(crate) struct UserActions<'ctx, T: ConnectionTrait> {
    connection: &'ctx T,
}

impl<'ctx, T> UserActions<'ctx, T>
where
    T: ConnectionTrait,
{
    /// Define what connection will be used
    pub(crate) fn new(connection: &'ctx T) -> Self {
        UserActions { connection }
    }

    /// Create a new user action
    pub(crate) async fn create(
        &self,
        user_action: user_actions::ActiveModel,
    ) -> AppResult<user_actions::Model> {
        let id = entity::active_value_to_uuid(user_action.id.clone())
            .ok_or_else(|| Error::BadRequest("user_action_no_id".to_string()))?;

        user_actions::Entity::insert(user_action)
            .exec_without_returning(self.connection)
            .await?;

        let select = user_actions::Entity::find_by_id(id);

        let result = select.one(self.connection).await?;

        result.ok_or(Error::NotFound("user_action_not_found".to_string()))
    }

    /// Generate new user action for user and specific action.
    pub(crate) async fn for_user(
        &self,
        user: &users::Model,
        action: &str,
    ) -> AppResult<user_actions::Model> {
        let id = entity::Uuid::new_v4();

        let active_model = user_actions::ActiveModel {
            id: entity::ActiveValue::Set(id),
            email: entity::ActiveValue::Set(user.email.clone()),
            action: entity::ActiveValue::Set(action.to_string()),
            user_id: entity::ActiveValue::Set(user.id),
            created_at: entity::ActiveValue::Set(chrono::Utc::now().timestamp()),
        };

        self.create(active_model).await
    }

    /// Find user action by id
    pub(crate) async fn get_by_id(
        &self,
        id: entity::Uuid,
    ) -> AppResult<(user_actions::Model, users::Model)> {
        let query = user_actions::Entity::find_by_id(id)
            .inner_join(users::Entity)
            .select_also(users::Entity);

        let result = query.one(self.connection).await?.ok_or(Error::NotFound(
            "user_action_not_found_or_executed".to_string(),
        ))?;

        // Its okay to unwrap here, we did inner_join
        Ok((result.0, result.1.unwrap()))
    }

    /// Delete user action after it has been executed
    pub(crate) async fn delete(&self, id: entity::Uuid) -> AppResult<()> {
        user_actions::Entity::delete_by_id(id)
            .exec(self.connection)
            .await?;

        Ok(())
    }
}
