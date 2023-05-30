use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{authenticated::Authenticated, credentials::Credentials},
};
use error::{AppResult, Error};

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
impl<'ctx> AuthProviderContract for CredentialsProvider<'ctx> {
    async fn authenticate(&self, user_agent: &str, ip: &str) -> AppResult<Authenticated> {
        let (email, password, token) = self.data.into_tuple()?;

        let user = match self.auth.get_by_email(&email).await {
            Ok(v) => v,
            Err(e) => {
                if e.is_not_found() {
                    return Err(Error::Unauthorized("invalid_credentials".to_string()));
                }

                return Err(e);
            }
        };

        if let Some(hashed_password) = &user.password {
            if !util::password::verify(&password, hashed_password) {
                return Err(Error::Unauthorized("invalid_credentials".to_string()));
            }
        } else {
            return Err(Error::Unauthorized("invalid_credentials".to_string()));
        }

        if let Some(secret) = &user.secret {
            if !util::validation::validate_otp(secret, token.as_ref()) {
                return Err(Error::Unauthorized("invalid_otp_token".to_string()));
            }
        }

        let session = self.auth.generate_session(&user, user_agent, ip).await?;

        Ok(Authenticated { user, session })
    }
}
