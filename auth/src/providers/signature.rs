use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{authenticated::Authenticated, signature::Signature},
};
use error::{AppResult, Error};

pub struct SignatureProvider<'ctx> {
    pub auth: &'ctx Auth<'ctx>,
    pub data: Signature,
}

impl<'ctx> SignatureProvider<'ctx> {
    pub fn new(auth: &'ctx Auth, data: Signature) -> Self {
        Self { auth, data }
    }
}

#[async_trait::async_trait]
impl<'ctx> AuthProviderContract for SignatureProvider<'ctx> {
    async fn authenticate(&self) -> AppResult<Authenticated> {
        let (fingerprint, signature, remember) = self.data.into_tuple()?;

        let user = match self.auth.get_by_fingerprint(&fingerprint).await {
            Ok(v) => v,
            Err(e) => {
                if e.is_not_found() {
                    return Err(Error::Unauthorized("invalid_signature".to_string()));
                }

                return Err(e);
            }
        };

        let nonce = Auth::generate_fingerprint_nonce(&user.fingerprint);

        cryptfns::rsa::public::verify(&nonce, &signature, &user.pubkey)?;

        let session = self.auth.generate_session(&user, remember).await?;

        Ok(Authenticated { user, session })
    }
}
