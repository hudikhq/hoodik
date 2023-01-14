use crate::data::CreateUser;
use entity::users;
use error::AppResult;

#[async_trait::async_trait]
pub trait AuthContract {
    async fn create(&self, data: CreateUser) -> AppResult<users::Model>;
    async fn get_by_id(&self, id: i32) -> AppResult<users::Model>;
}
