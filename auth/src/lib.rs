use context::Context;
use contract::AuthContract;
use error::{AppResult, Error};
use store::{user, ActiveModelTrait, EntityTrait};

pub mod contract;
pub mod data;

pub struct Auth<'ctx> {
    pub context: &'ctx Context,
}

#[async_trait::async_trait]
impl<'ctx> AuthContract for Auth<'ctx> {
    async fn create_user(&self, active_model: user::ActiveModel) -> AppResult<user::Model> {
        active_model
            .insert(&self.context.db)
            .await
            .map_err(Error::from)
    }

    async fn get_user(&self, id: i32) -> AppResult<user::Model> {
        user::Entity::find_by_id(id)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", id)))
    }
}
