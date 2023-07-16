use context::Context;
use error::AppResult;

/// Expose the inner context to the implementor
#[async_trait::async_trait]
pub(crate) trait Ctx {
    fn ctx(&self) -> &Context;

    /// Check if email activation is enforced
    async fn enforce_email_activation(&self) -> bool {
        self.ctx()
            .settings
            .inner()
            .await
            .users
            .enforce_email_activation()
    }

    /// Check if registration is allowed for the given email,
    /// and throw with the given error if not.
    async fn can_register_or_else<T: FnOnce() -> AppResult<()> + Send + Sync>(
        &self,
        email: &str,
        i: T,
    ) -> AppResult<()> {
        self.ctx()
            .settings
            .inner()
            .await
            .users
            .can_register_or_else(email, i)
    }
}
