use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use sea_orm::error::{ColumnFromStrErr, DbErr, RuntimeErr};
use serde::Serialize;
use thiserror::Error as ThisError;
use validr::error::ValidationErrors;

pub type AppResult<T> = Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    NotFound(String),
    DbErr(DbErr),
    RuntimeErr(RuntimeErr),
    ColumnFromStrErr(ColumnFromStrErr),
    BadRequest(String),
    Validation(ValidationErrors),
    Unauthorized(String),
    InternalError(String),
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::NotFound(_))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {:?}", self)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl From<DbErr> for Error {
    fn from(source: DbErr) -> Error {
        Error::DbErr(source)
    }
}

impl From<RuntimeErr> for Error {
    fn from(source: RuntimeErr) -> Error {
        Error::RuntimeErr(source)
    }
}

impl From<ColumnFromStrErr> for Error {
    fn from(source: ColumnFromStrErr) -> Error {
        Error::ColumnFromStrErr(source)
    }
}

impl From<ValidationErrors> for Error {
    fn from(source: ValidationErrors) -> Error {
        Error::Validation(source)
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    #[serde(skip_serializing)]
    pub status: u16,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

impl From<&Error> for ErrorResponse {
    fn from(source: &Error) -> ErrorResponse {
        match source {
            Error::NotFound(message) => ErrorResponse {
                status: 404,
                message: message.clone(),
                context: None,
            },
            Error::DbErr(err) => ErrorResponse {
                status: 500,
                message: err.to_string(),
                context: None,
            },
            Error::RuntimeErr(err) => ErrorResponse {
                status: 500,
                message: err.to_string(),
                context: None,
            },
            Error::ColumnFromStrErr(err) => ErrorResponse {
                status: 500,
                message: err.to_string(),
                context: None,
            },
            Error::BadRequest(err) => ErrorResponse {
                status: 400,
                message: err.to_string(),
                context: None,
            },
            Error::Validation(err) => ErrorResponse {
                status: 422,
                message: "Validation error".to_string(),
                context: Some(serde_json::to_value(err).unwrap()),
            },
            Error::Unauthorized(message) => ErrorResponse {
                status: 401,
                message: message.clone(),
                context: None,
            },
            Error::InternalError(message) => ErrorResponse {
                status: 500,
                message: message.clone(),
                context: None,
            },
        }
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        let payload = ErrorResponse::from(self);

        HttpResponseBuilder::new(actix_web::http::StatusCode::from_u16(payload.status).unwrap())
            .json(payload)
    }
}
