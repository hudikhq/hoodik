use chrono::Utc;
use error::{AppResult, Error};
use log::error;
use serde::{Deserialize, Serialize};

use crate::data::authenticated::Authenticated;

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub authenticated: Authenticated,
    pub exp: i64,
    pub iat: i64,
}

/// Generate JWT token
pub fn generate(authenticated: &Authenticated, secret: &str) -> AppResult<String> {
    if secret.is_empty() {
        error!("Generating unsecure JWT without secret set!");
    }

    let exp = authenticated.session.expires_at.timestamp();

    let claims = Claims {
        sub: String::from(&authenticated.session.token),
        email: String::from(&authenticated.user.email),
        iat: Utc::now().timestamp(),
        authenticated: authenticated.clone(),
        exp,
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(Error::from)
}

/// Extract and verify given token and return authenticated data
pub fn extract(claims: &str, secret: &str) -> AppResult<Authenticated> {
    if secret.is_empty() {
        error!("Generating unsecure JWT without secret set!");
    }

    jsonwebtoken::decode::<Claims>(
        claims,
        &jsonwebtoken::DecodingKey::from_secret(secret.as_bytes()),
        &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .map_err(Error::from)
    .map(|data| data.claims.authenticated)
}
