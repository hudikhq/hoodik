use std::pin::Pin;

use actix_web::FromRequest;
use error::{AppResult, Error};
use futures_util::Future;

use super::claims::Claims;

pub struct Staff {
    pub claims: Claims,
}

impl Staff {
    pub fn is_expired(&self) -> bool {
        self.claims.is_expired()
    }

    pub fn is_valid(&self) -> bool {
        self.claims.is_valid()
    }

    pub fn is_admin(&self) -> bool {
        self.claims.role == Some("admin".to_string())
    }

    pub fn is_admin_or_err(&self) -> AppResult<()> {
        if self.is_admin() {
            return Ok(());
        }

        Err(Error::Forbidden("auth::data::staff|not_admin".to_string()))
    }
}

impl FromRequest for Staff {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let fut = Claims::extract(req);

        Box::pin(async {
            let claims = fut.await?;

            if claims.role.is_none() {
                return Err(Error::Forbidden("auth::data::staff|not_staff".to_string()));
            }

            Ok(Self { claims })
        })
    }

    fn extract(req: &actix_web::HttpRequest) -> Self::Future {
        Self::from_request(req, &mut actix_web::dev::Payload::None)
    }
}
