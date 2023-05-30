use context::Context;
use entity::{user_actions, users, ConnectionTrait, EntityTrait};
use error::{AppResult, Error};

pub(crate) struct UserActions<'ctx, T: ConnectionTrait> {
    context: &'ctx Context,
    connection: Option<&'ctx T>,
}

impl<'ctx, T> UserActions<'ctx, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            connection: None,
        }
    }

    /// Define what connection will be used
    pub(crate) fn with_connection(mut self, connection: &'ctx T) -> Self {
        self.connection = Some(connection);

        self
    }

    /// Create a new user action
    pub(crate) async fn create(
        &self,
        user_action: user_actions::ActiveModel,
    ) -> AppResult<user_actions::Model> {
        let id = entity::active_value_to_uuid(user_action.id.clone())
            .ok_or_else(|| Error::BadRequest("user_action_no_id".to_string()))?;

        user_actions::Entity::insert(user_action)
            .exec_without_returning(&self.context.db)
            .await?;

        let select = user_actions::Entity::find_by_id(id);

        let result = match self.connection {
            Some(c) => select.one(c).await?,
            None => select.one(&self.context.db).await?,
        };

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
            created_at: entity::ActiveValue::Set(chrono::Utc::now().naive_utc()),
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

        let result = match self.connection {
            Some(c) => query.one(c).await?,
            None => query.one(&self.context.db).await?,
        }
        .ok_or(Error::NotFound(
            "user_action_not_found_or_executed".to_string(),
        ))?;

        // Its okay to unwrap here, we did inner_join
        Ok((result.0, result.1.unwrap()))
    }

    /// Delete user action after it has been executed
    pub(crate) async fn delete(&self, id: entity::Uuid) -> AppResult<()> {
        let statement = user_actions::Entity::delete_by_id(id);

        match self.connection {
            Some(c) => statement.exec(c).await?,
            None => statement.exec(&self.context.db).await?,
        };

        Ok(())
    }
}
