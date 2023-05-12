use error::{AppResult, Error};
use log::error;

use crate::data::{authenticated::Authenticated, claims::Claims};

/// Generate JWT token
pub fn generate(authenticated: &Authenticated, issuer: &str, secret: &str) -> AppResult<String> {
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
pub fn extract(claims: &str, secret: &str) -> AppResult<Claims> {
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
