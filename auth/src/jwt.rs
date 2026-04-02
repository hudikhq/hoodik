use error::{AppResult, Error};
use log::error;

use crate::data::{authenticated::Authenticated, claims::Claims, transfer_claims::TransferClaims};

/// Generate JWT token
pub(crate) fn generate(
    authenticated: &Authenticated,
    issuer: &str,
    secret: &str,
) -> AppResult<String> {
    if secret.is_empty() {
        error!("Generating unsecure JWT without secret set!");
    }

    let claims = Claims::from(authenticated).set_iss(issuer);

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(Error::from)
}

/// Extract and verify given token and return authenticated data
pub(crate) fn extract(claims: &str, secret: &str) -> AppResult<Claims> {
    if secret.is_empty() {
        error!("Generating unsecure JWT without secret set!");
    }

    let mut validator = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validator.validate_exp = false;

    jsonwebtoken::decode::<Claims>(
        claims,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validator,
    )
    .map_err(Error::from)
    .map(|data| data.claims)
}

/// Generate a transfer token JWT scoped to a specific file and action.
pub(crate) fn generate_transfer_token(
    claims: &TransferClaims,
    secret: &str,
) -> AppResult<String> {
    if secret.is_empty() {
        error!("Generating unsecure JWT without secret set!");
    }

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(Error::from)
}

/// Extract and verify a transfer token JWT.
pub(crate) fn extract_transfer_claims(token: &str, secret: &str) -> AppResult<TransferClaims> {
    if secret.is_empty() {
        error!("Generating unsecure JWT without secret set!");
    }

    let mut validator = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validator.validate_exp = false;

    jsonwebtoken::decode::<TransferClaims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &validator,
    )
    .map_err(Error::from)
    .map(|data| data.claims)
}
