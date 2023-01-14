use error::AppResult;
use store::user;

#[async_trait::async_trait]
pub trait AuthContract {
    async fn create_user(&self, active_model: user::ActiveModel) -> AppResult<user::Model>;
    async fn get_user(&self, id: i32) -> AppResult<user::Model>;
}
