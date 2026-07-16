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

/// Widest clock skew accepted between the client-signed timestamp and the
/// server clock. Also bounds how long a spent nonce must stay recorded.
const CLIENT_NONCE_MAX_SKEW_SECONDS: i64 = 300;

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

    /// Canonical bytes a client-nonce login signs. Rebuilt here from the
    /// received fields so a client can never substitute a different blob the
    /// signature happens to match.
    fn client_nonce_canonical(fingerprint: &str, timestamp: i64, nonce: &str) -> String {
        format!("{fingerprint}:{timestamp}:{nonce}")
    }
}

#[async_trait::async_trait]
impl AuthProvider for SignatureProvider<'_> {
    async fn authenticate(&self, user_agent: &str, ip: &str) -> AppResult<Authenticated> {
        let (fingerprint, signature) = self.data.into_tuple()?;

        // Upgraded clients sign `fingerprint:timestamp:nonce` with a random
        // nonce, so back-to-back logins with the same key stay distinguishable
        // from replays. Clients predating those fields sign the deterministic
        // minute bucket instead; that path is kept accepted and carries its
        // known limitation that a second login in the same bucket is refused.
        // Either canonical is built from the *presented* fingerprint (the
        // client may send an old one from a pre-migration backup key).
        let (message, nonce, expires_at) = match (self.data.timestamp, self.data.nonce.as_deref())
        {
            (Some(timestamp), Some(nonce)) if !nonce.is_empty() => {
                if (Utc::now().timestamp() - timestamp).abs() > CLIENT_NONCE_MAX_SKEW_SECONDS {
                    return Err(Error::Unauthorized("signature_expired".to_string()));
                }

                (
                    Self::client_nonce_canonical(&fingerprint, timestamp, nonce),
                    nonce.to_string(),
                    timestamp + CLIENT_NONCE_MAX_SKEW_SECONDS,
                )
            }
            _ => {
                let nonce = Self::generate_fingerprint_nonce(&fingerprint);
                let expires_at = (Utc::now().timestamp() / 60 + 1) * 60;

                (nonce.clone(), nonce, expires_at)
            }
        };

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
            .verify(&message, &signature, &verify_pubkey)?;

        // Recording the verified nonce makes a captured request single-use;
        // the row only has to outlive the window in which the signature is
        // still acceptable — the timestamp skew for client nonces, the minute
        // bucket for legacy ones.
        if !self.auth.consume_login_nonce(&fingerprint, &nonce, expires_at).await? {
            return Err(Error::Unauthorized("signature_replayed".to_string()));
        }

        let session = self.auth.generate(&user, user_agent, ip).await?;

        Ok(Authenticated { user, session })
    }
}
