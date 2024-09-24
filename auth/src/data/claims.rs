//! # Authenticated Claims data
//! This data is the result of the JWT object provided by the user
//! either in the header or cookie.
use actix_web::{web, FromRequest, HttpRequest};
use context::Context;
use entity::Uuid;
use error::Error;
use futures_util::Future;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use super::{authenticated::Authenticated, extractor::Extractor};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claims {
    /// Issuer - authentication service provider
    pub iss: String,
    /// Subject - user id
    pub sub: Uuid,
    /// Expires at
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// Authenticated device id
    pub device: Uuid,
    /// User role
    pub role: Option<String>,
    /// User quota for the storage
    pub quota: Option<i64>,
}

impl From<&Authenticated> for Claims {
    fn from(authenticated: &Authenticated) -> Self {
        Self {
            iss: String::from("fresh"),
            sub: authenticated.user.id,
            exp: authenticated.session.expires_at,
            iat: chrono::Utc::now().timestamp(),
            device: authenticated.session.device_id,
            role: authenticated.user.role.clone(),
            quota: authenticated.user.quota,
        }
    }
}

impl Claims {
    pub fn set_iss<T: Into<String>>(mut self, iss: T) -> Self {
        self.iss = iss.into();

        self
    }

    pub fn is_expired(&self) -> bool {
        self.exp < chrono::Utc::now().timestamp()
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    pub async fn get_quota(&self, context: &Context) -> Option<u64> {
        match self.quota {
            Some(v) => Some(v as u64),
            None => {
                let settings = context.settings.inner().await;

                settings.users.quota_bytes()
            }
        }
    }
}

impl TryFrom<&HttpRequest> for Claims {
    type Error = Error;

    fn try_from(req: &HttpRequest) -> Result<Self, Error> {
        let context = match req.app_data::<web::Data<Context>>() {
            Some(c) => c,
            None => {
                return Err(Error::Unauthorized(
                    "auth::data::claims|no_context".to_string(),
                ));
            }
        };

        Extractor::default()
            .jwt(context)
            .req(req)
            .map_err(|e| {
                log::debug!("auth::data::claims|no_jwt: {:?}", e);

                Error::Unauthorized("auth::data::claims|no_claims".to_string())
            })
    }
}

impl FromRequest for Claims {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let claims = match Claims::try_from(req) {
            Ok(c) => c,
            Err(e) => return Box::pin(async { Err(e) }),
        };

        if claims.is_expired() {
            return Box::pin(async {
                Err(Error::Unauthorized(
                    "auth::data::claims|expired".to_string(),
                ))
            });
        }

        Box::pin(async move { Ok(claims) })
    }

    fn extract(req: &actix_web::HttpRequest) -> Self::Future {
        Self::from_request(req, &mut actix_web::dev::Payload::None)
    }
}
