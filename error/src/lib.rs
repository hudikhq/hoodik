use std::string::FromUtf8Error;

use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use base64::DecodeError;
use hex::FromHexError;
use rsa::{
    errors::Error as RSAError, pkcs1::Error as PKCS1Error, pkcs8::spki::Error as SpkiError,
    pkcs8::Error as PKCS8Error, signature::Error as SignatureError,
};
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
    RSAError(RSAError),
    PKCS1Error(PKCS1Error),
    PKCS8Error(PKCS8Error),
    PKCS8SpkiError(SpkiError),
    SignatureError(SignatureError),
    Base64DecodeError(DecodeError),
    HexDecodeError(FromHexError),
    FromUtf8Error(FromUtf8Error),
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

impl From<RSAError> for Error {
    fn from(source: RSAError) -> Error {
        Error::RSAError(source)
    }
}

impl From<SignatureError> for Error {
    fn from(source: SignatureError) -> Error {
        Error::SignatureError(source)
    }
}

impl From<PKCS1Error> for Error {
    fn from(source: PKCS1Error) -> Error {
        Error::PKCS1Error(source)
    }
}

impl From<PKCS8Error> for Error {
    fn from(source: PKCS8Error) -> Error {
        Error::PKCS8Error(source)
    }
}

impl From<SpkiError> for Error {
    fn from(source: SpkiError) -> Error {
        Error::PKCS8SpkiError(source)
    }
}

impl From<DecodeError> for Error {
    fn from(source: DecodeError) -> Error {
        Error::Base64DecodeError(source)
    }
}

impl From<FromHexError> for Error {
    fn from(source: FromHexError) -> Error {
        Error::HexDecodeError(source)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(source: FromUtf8Error) -> Error {
        Error::FromUtf8Error(source)
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
            Error::RSAError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::SignatureError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::PKCS1Error(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::PKCS8Error(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::PKCS8SpkiError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::Base64DecodeError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::HexDecodeError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::FromUtf8Error(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
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
