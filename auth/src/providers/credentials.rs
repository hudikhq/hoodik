use crate::{
    auth::Auth,
    contracts::{ctx::Ctx, provider::AuthProvider, repository::Repository, sessions::Sessions},
    data::{authenticated::Authenticated, credentials::Credentials},
};
use error::{AppResult, Error};

/// Authentication provider for logging in with credentials (email + password)
pub(crate) struct CredentialsProvider<'ctx> {
    auth: &'ctx Auth<'ctx>,
    data: Credentials,
}

impl<'ctx> CredentialsProvider<'ctx> {
    pub(crate) fn new(auth: &'ctx Auth, data: Credentials) -> Self {
        Self { auth, data }
    }
}

#[async_trait::async_trait]
impl AuthProvider for CredentialsProvider<'_> {
    async fn authenticate(&self, user_agent: &str, ip: &str) -> AppResult<Authenticated> {
        let (email, password, token) = self.data.into_tuple()?;

        let mut user = match self.auth.get_by_email(&email).await {
            Ok(v) => v,
            Err(e) => {
                if e.is_not_found() {
                    return Err(Error::Unauthorized("invalid_credentials".to_string()));
                }

                return Err(e);
            }
        };

        if user.quota.is_none() {
            user.quota = self
                .auth
                .context
                .settings
                .inner()
                .await
                .users
                .quota_bytes()
                .map(|v| v as i64);
        }

        if let Some(hashed_password) = &user.password {
            if !util::password::verify(&password, hashed_password) {
                return Err(Error::Unauthorized("invalid_credentials".to_string()));
            }
        } else {
            return Err(Error::Unauthorized("invalid_credentials".to_string()));
        }

        if !user.verify_tfa(token) {
            return Err(Error::Unauthorized("invalid_otp_token".to_string()));
        }

        if self.auth.enforce_email_activation().await && user.email_verified_at.is_none() {
            return Err(Error::Unauthorized("inactive_account".to_string()));
        }

        let session = self.auth.generate(&user, user_agent, ip).await?;

        Ok(Authenticated { user, session })
    }
}
