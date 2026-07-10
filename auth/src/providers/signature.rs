use crate::{
    auth::Auth,
    contracts::{provider::AuthProvider, repository::Repository, sessions::Sessions},
    data::{authenticated::Authenticated, signature::Signature},
};
use chrono::Utc;
use cryptfns::identity::KeyType;
use error::{AppResult, Error};
use std::str::FromStr;

/// Authentication provider for authentication with private key
pub(crate) struct SignatureProvider<'ctx> {
    auth: &'ctx Auth<'ctx>,
    data: Signature,
}

impl<'ctx> SignatureProvider<'ctx> {
    pub(crate) fn new(auth: &'ctx Auth, data: Signature) -> Self {
        Self { auth, data }
    }

    pub(crate) fn generate_nonce_minutes() -> String {
        format!("{}", Utc::now().timestamp() / 60)
    }

    pub(crate) fn generate_fingerprint_nonce(fingerprint: &str) -> String {
        format!("{}-{}", fingerprint, Self::generate_nonce_minutes())
    }
}

#[async_trait::async_trait]
impl AuthProvider for SignatureProvider<'_> {
    async fn authenticate(&self, user_agent: &str, ip: &str) -> AppResult<Authenticated> {
        let (fingerprint, signature) = self.data.into_tuple()?;

        // Build nonce from the *presented* fingerprint (client may send an old one
        // from a pre-migration backup key).
        let nonce = Self::generate_fingerprint_nonce(&fingerprint);

        // Resolve the user by the presented fingerprint, or fall back to an old
        // fingerprint recorded in a key transition. Drop the lookup error before
        // any await — its type is !Send and must not live across one. When the
        // fallback is taken, the same transition row carries the historical key
        // the signature verifies against, so fetch it once and reuse it below.
        let fp_lookup = self.auth.get_by_fingerprint(&fingerprint).await.ok();
        let (mut user, transition) = if let Some(u) = fp_lookup {
            (u, None)
        } else {
            let trans = self
                .auth
                .get_key_transition_by_old_fingerprint(&fingerprint)
                .await?
                .ok_or_else(|| Error::Unauthorized("invalid_signature".to_string()))?;
            let user = self.auth.get_by_id(trans.user_id).await?;
            (user, Some(trans))
        };

        // Verify against the current key when the presented fingerprint is live,
        // otherwise against the superseded key rebuilt from the transition row,
        // using the algorithm it was recorded with.
        let (verify_key_type, verify_pubkey) = match &transition {
            None => (user.key_type.clone(), user.pubkey.clone()),
            Some(trans) => {
                let old_key_type = KeyType::from_str(&trans.old_key_type)?;
                let pem = old_key_type
                    .pem_from_member_der(&trans.old_key_spki)
                    .map_err(|_| Error::Unauthorized("invalid_signature".to_string()))?;
                (trans.old_key_type.clone(), pem)
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

        KeyType::from_str(&verify_key_type)?
            .verify(&nonce, &signature, &verify_pubkey)?;

        let session = self.auth.generate(&user, user_agent, ip).await?;

        Ok(Authenticated { user, session })
    }
}
