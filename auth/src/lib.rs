use context::Context;
use contract::AuthContract;
use data::CreateUser;
use entity::{users, ActiveModelTrait, EntityTrait};
use error::{AppResult, Error};
use validr::Validation;

pub mod contract;
pub mod data;

pub struct Auth<'ctx> {
    pub context: &'ctx Context,
}

#[async_trait::async_trait]
impl<'ctx> AuthContract for Auth<'ctx> {
    async fn create(&self, data: CreateUser) -> AppResult<users::Model> {
        let active_model = data.validate()?.into_active_model();

        active_model
            .insert(&self.context.db)
            .await
            .map_err(Error::from)
    }

    async fn get_by_id(&self, id: i32) -> AppResult<users::Model> {
        users::Entity::find_by_id(id)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", id)))
    }
}

#[cfg(test)]
mod test;
