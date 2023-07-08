use crate::data::authenticated::Authenticated;
use error::AppResult;

/// Authentication provider
#[async_trait::async_trait]
pub(crate) trait AuthProvider {
    /// Authentication method that has to be implemented on the providers
    /// that will handle their own authentication methods
    async fn authenticate(&self, user_agent: &str, ip: &str) -> AppResult<Authenticated>;
}
