use crate::data::authenticated::Authenticated;
use error::AppResult;

#[async_trait::async_trait]
pub trait AuthProviderContract {
    /// Authentication method that has to be implemented on the providers
    /// that will handle their own authentication methods
    async fn authenticate(&self) -> AppResult<Authenticated>;
}
