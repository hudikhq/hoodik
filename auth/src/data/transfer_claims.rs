//! # Transfer Claims
//!
//! Long-lived JWT claims scoped to a specific file transfer operation
//! (upload or download). These tokens allow mobile clients to continue
//! transfers even when the short-lived session JWT has expired.

use actix_web::{web, FromRequest, HttpRequest};
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use futures_util::Future;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use super::claims::Claims;

/// JWT claims for a transfer token. Scoped to a single file and action.
///
/// The `path` field encodes both the action and file ID as
/// `"upload/{file_id}"` or `"download/{file_id}"`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransferClaims {
    /// Issuer — always `"transfer"` for transfer tokens.
    pub iss: String,
    /// Subject — the user ID that owns the transfer.
    pub sub: Uuid,
    /// Expiration timestamp (unix seconds). Set to
    /// `now + long_term_session_duration_days` from config.
    pub exp: i64,
    /// Issued-at timestamp (unix seconds).
    pub iat: i64,
    /// Scoped path: `"upload/{file_id}"` or `"download/{file_id}"`.
    pub path: String,
}

impl TransferClaims {
    pub fn is_expired(&self) -> bool {
        self.exp < chrono::Utc::now().timestamp()
    }

    /// Validate that this token's path matches the expected action and file ID.
    pub fn validate_path(&self, file_id: Uuid, action: &str) -> AppResult<()> {
        let expected = format!("{}/{}", action, file_id);
        if self.path != expected {
            return Err(Error::Forbidden(
                "transfer_token_path_mismatch".to_string(),
            ));
        }
        Ok(())
    }
}

/// Unified extractor for storage routes — accepts either a regular session
/// `Claims` (from cookie/header) or a `TransferClaims` (from the
/// `Authorization: Bearer` header with `iss == "transfer"`).
pub enum StorageClaims {
    Session(Claims),
    Transfer(TransferClaims),
}

impl StorageClaims {
    /// The authenticated user ID.
    pub fn sub(&self) -> Uuid {
        match self {
            Self::Session(c) => c.sub,
            Self::Transfer(tc) => tc.sub,
        }
    }

    /// Get the user's storage quota. Transfer tokens don't carry quota —
    /// quota is only enforced during file creation, not chunk upload/download.
    pub async fn get_quota(&self, context: &Context) -> Option<u64> {
        match self {
            Self::Session(c) => c.get_quota(context).await,
            Self::Transfer(_) => None,
        }
    }

    /// For transfer tokens, validates the path matches the expected file ID
    /// and action. For session tokens, always succeeds.
    pub fn validate_transfer_path(&self, file_id: Uuid, action: &str) -> AppResult<()> {
        match self {
            Self::Session(_) => Ok(()),
            Self::Transfer(tc) => tc.validate_path(file_id, action),
        }
    }
}

impl FromRequest for StorageClaims {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    fn from_request(
        req: &HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        // Try to extract a transfer token from the Authorization header first.
        let transfer_result = try_extract_transfer_claims(req);

        match transfer_result {
            Ok(tc) => Box::pin(async move { Ok(StorageClaims::Transfer(tc)) }),
            Err(_) => {
                // Fall back to regular Claims extraction (cookie or header).
                let claims_fut = Claims::from_request(req, payload);
                Box::pin(async move {
                    let claims = claims_fut.await?;
                    Ok(StorageClaims::Session(claims))
                })
            }
        }
    }

    fn extract(req: &HttpRequest) -> Self::Future {
        Self::from_request(req, &mut actix_web::dev::Payload::None)
    }
}

/// Try to extract `TransferClaims` from the `Authorization: Bearer` header.
///
/// Returns `Ok(TransferClaims)` only if the header is present, the JWT is
/// valid, the `iss` field is `"transfer"`, and the token is not expired.
/// Otherwise returns `Err`.
fn try_extract_transfer_claims(req: &HttpRequest) -> AppResult<TransferClaims> {
    let context = req
        .app_data::<web::Data<Context>>()
        .ok_or_else(|| Error::Unauthorized("no_context".to_string()))?;

    let header = req
        .headers()
        .get("Authorization")
        .ok_or_else(|| Error::Unauthorized("no_authorization_header".to_string()))?
        .to_str()
        .map_err(|_| Error::Unauthorized("invalid_authorization_header".to_string()))?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Error::Unauthorized("invalid_authorization_header".to_string()))?;

    let tc = crate::jwt::extract_transfer_claims(token, &context.config.auth.jwt_secret)?;

    if tc.iss != "transfer" {
        return Err(Error::Unauthorized("not_a_transfer_token".to_string()));
    }

    if tc.is_expired() {
        return Err(Error::Unauthorized("transfer_token_expired".to_string()));
    }

    Ok(tc)
}
