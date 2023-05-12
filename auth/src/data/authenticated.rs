use std::pin::Pin;

use actix_web::{web, FromRequest};
use context::Context;
use entity::{sessions, users};
use error::Error;
use futures_util::Future;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

use super::claims::Claims;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Authenticated {
    pub user: users::Model,
    pub session: sessions::Model,
}

impl Authenticated {
    pub fn is_expired(&self) -> bool {
        self.session.expires_at.timestamp() < chrono::Utc::now().timestamp()
    }
}

impl FromRequest for Authenticated {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let context = match req.app_data::<web::Data<Context>>() {
            Some(c) => c.clone(),
            None => {
                log::debug!("auth::data::authenticated|no_context request extract attempt");

                return Box::pin(async {
                    Err(Error::Unauthorized(
                        "auth::data::authenticated|no_context".to_string(),
                    ))
                });
            }
        };

        let claims = match Claims::try_from(req) {
            Ok(c) => c,
            Err(e) => return Box::pin(async { Err(e) }),
        };

        if claims.is_expired() {
            return Box::pin(async {
                Err(Error::Unauthorized(
                    "auth::data::authenticated|claims_expired".to_string(),
                ))
            });
        }

        Box::pin(async move {
            let authenticated = match Auth::new(&context).get_by_device_id(claims.device).await {
                Ok(a) => a,
                Err(e) => return Err(e),
            };

            if authenticated.is_expired() {
                return Err(Error::Unauthorized(
                    "auth::data::authenticated|claims_expired".to_string(),
                ));
            }

            Ok(authenticated)
        })
    }

    fn extract(req: &actix_web::HttpRequest) -> Self::Future {
        Self::from_request(req, &mut actix_web::dev::Payload::None)
    }
}
