use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{authenticated::Authenticated, signature::Signature},
};
use error::{AppResult, Error};

pub(crate) struct SignatureProvider<'ctx> {
    auth: &'ctx Auth<'ctx>,
    data: Signature,
}

impl<'ctx> SignatureProvider<'ctx> {
    pub(crate) fn new(auth: &'ctx Auth, data: Signature) -> Self {
        Self { auth, data }
    }
}

#[async_trait::async_trait]
impl<'ctx> AuthProviderContract for SignatureProvider<'ctx> {
    async fn authenticate(&self, user_agent: &str, ip: &str) -> AppResult<Authenticated> {
        let (fingerprint, signature) = self.data.into_tuple()?;

        let mut user = match self.auth.get_by_fingerprint(&fingerprint).await {
            Ok(v) => v,
            Err(e) => {
                if e.is_not_found() {
                    return Err(Error::Unauthorized("invalid_signature".to_string()));
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

        let nonce = Auth::generate_fingerprint_nonce(&user.fingerprint);

        cryptfns::rsa::public::verify(&nonce, &signature, &user.pubkey)?;

        let session = self.auth.generate_session(&user, user_agent, ip).await?;

        Ok(Authenticated { user, session })
    }
}
