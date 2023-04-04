use actix_web::{HttpMessage, HttpRequest};
use entity::{sessions, users};
use error::Error as AppError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Authenticated {
    pub user: users::Model,
    pub session: sessions::Model,
}

impl TryFrom<&HttpRequest> for Authenticated {
    type Error = AppError;

    fn try_from(req: &HttpRequest) -> Result<Self, AppError> {
        match req.extensions().get::<Authenticated>() {
            Some(authenticated) => Ok(authenticated.clone()),
            None => {
                log::debug!("auth::data::authenticated|no_session request extract attempt");

                Err(AppError::Unauthorized("no_session".to_string()))
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedJwt {
    pub authenticated: Authenticated,
    pub jwt: String,
}
