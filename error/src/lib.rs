use actix_multipart::MultipartError;
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use base64::DecodeError;
use cryptfns::error::Error as CryptoError;
use glob::{GlobError, PatternError};
use handlebars::{RenderError, TemplateError};
use hex::FromHexError;
use jsonwebtoken::errors::Error as JWTError;
use lettre::{
    address::AddressError, error::Error as LettreError, transport::smtp::Error as SmtpError,
};
use rcgen::RcgenError;
use reqwest::Error as ReqwestError;
use rustls::Error as RustlsError;
use sea_orm::{
    error::{ColumnFromStrErr, DbErr, RuntimeErr},
    TransactionError,
};
use serde::Serialize;
use serde_json::Error as SerdeJsonError;
use std::io::Error as IoError;
use std::string::FromUtf8Error;
use thiserror::Error as ThisError;
use uuid::Error as UuidError;
use validr::error::{ValidationError, ValidationErrors};

pub type AppResult<T> = Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    NotFound(String),
    DbErr(Box<DbErr>),
    RuntimeErr(Box<RuntimeErr>),
    ColumnFromStrErr(Box<ColumnFromStrErr>),
    BadRequest(String),
    Validation(ValidationErrors),
    Unauthorized(String),
    Forbidden(String),
    InternalError(String),
    CryptoError(Box<CryptoError>),
    Base64DecodeError(Box<DecodeError>),
    HexDecodeError(Box<FromHexError>),
    FromUtf8Error(Box<FromUtf8Error>),
    JWTError(Box<JWTError>),
    ReqwestError(Box<ReqwestError>),
    StorageError(String),
    MultipartError(Box<MultipartError>),
    SerdeJsonError(Box<SerdeJsonError>),
    UuidError(Box<UuidError>),
    RustlsError(Box<RustlsError>),
    RcgenError(Box<RcgenError>),
    SmtpError(Box<SmtpError>),
    LettreError(Box<LettreError>),
    AddressError(Box<AddressError>),
    HandlebarsRenderError(Box<RenderError>),
    HandlebarsTemplateError(Box<TemplateError>),
    TooManyRequests(String),
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        matches!(self, Error::NotFound(_))
    }

    pub fn as_wrong_id(entity: &str) -> Error {
        Error::BadRequest(format!("invalid_id_provided_while_extracting:{entity}"))
    }

    pub fn as_not_found(message: &str) -> Error {
        Error::NotFound(message.to_string())
    }

    pub fn as_validation(field: &str, message: &str) -> Error {
        let mut errors = ValidationErrors::new();
        let mut error = ValidationError::new();
        error.set_field_name(field);
        error.add(message);
        errors.add(error);

        Error::Validation(errors)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {self:?}")
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl From<Box<dyn std::any::Any + Send>> for Error {
    fn from(source: Box<dyn std::any::Any + Send>) -> Error {
        Error::InternalError(format!("{source:?}"))
    }
}

impl From<&str> for Error {
    fn from(source: &str) -> Error {
        Error::InternalError(source.to_string())
    }
}

impl From<String> for Error {
    fn from(source: String) -> Error {
        Error::from(source.as_str())
    }
}

impl From<DbErr> for Error {
    fn from(source: DbErr) -> Error {
        Error::DbErr(Box::new(source))
    }
}

impl From<TransactionError<Error>> for Error {
    fn from(source: TransactionError<Error>) -> Error {
        match source {
            TransactionError::Connection(err) => Error::DbErr(Box::new(err)),
            TransactionError::Transaction(err) => err,
        }
    }
}

impl From<RuntimeErr> for Error {
    fn from(source: RuntimeErr) -> Error {
        Error::RuntimeErr(Box::new(source))
    }
}

impl From<ColumnFromStrErr> for Error {
    fn from(source: ColumnFromStrErr) -> Error {
        Error::ColumnFromStrErr(Box::new(source))
    }
}

impl From<ValidationErrors> for Error {
    fn from(source: ValidationErrors) -> Error {
        Error::Validation(source)
    }
}

impl From<CryptoError> for Error {
    fn from(source: CryptoError) -> Error {
        Error::CryptoError(Box::new(source))
    }
}

impl From<DecodeError> for Error {
    fn from(source: DecodeError) -> Error {
        Error::Base64DecodeError(Box::new(source))
    }
}

impl From<FromHexError> for Error {
    fn from(source: FromHexError) -> Error {
        Error::HexDecodeError(Box::new(source))
    }
}

impl From<FromUtf8Error> for Error {
    fn from(source: FromUtf8Error) -> Error {
        Error::FromUtf8Error(Box::new(source))
    }
}

impl From<JWTError> for Error {
    fn from(source: JWTError) -> Error {
        Error::JWTError(Box::new(source))
    }
}

impl From<ReqwestError> for Error {
    fn from(source: ReqwestError) -> Error {
        Error::ReqwestError(Box::new(source))
    }
}

impl From<IoError> for Error {
    fn from(source: IoError) -> Error {
        Error::StorageError(source.to_string())
    }
}

impl From<PatternError> for Error {
    fn from(source: PatternError) -> Error {
        Error::StorageError(source.to_string())
    }
}

impl From<GlobError> for Error {
    fn from(source: GlobError) -> Error {
        Error::StorageError(source.to_string())
    }
}

impl From<MultipartError> for Error {
    fn from(source: MultipartError) -> Error {
        Error::MultipartError(Box::new(source))
    }
}

impl From<SerdeJsonError> for Error {
    fn from(source: SerdeJsonError) -> Error {
        Error::SerdeJsonError(Box::new(source))
    }
}

impl From<UuidError> for Error {
    fn from(source: UuidError) -> Error {
        Error::UuidError(Box::new(source))
    }
}

impl From<RustlsError> for Error {
    fn from(source: RustlsError) -> Error {
        Error::RustlsError(Box::new(source))
    }
}

impl From<RcgenError> for Error {
    fn from(source: RcgenError) -> Error {
        Error::RcgenError(Box::new(source))
    }
}

impl From<LettreError> for Error {
    fn from(source: LettreError) -> Error {
        Error::LettreError(Box::new(source))
    }
}

impl From<SmtpError> for Error {
    fn from(source: SmtpError) -> Error {
        Error::SmtpError(Box::new(source))
    }
}

impl From<AddressError> for Error {
    fn from(source: AddressError) -> Error {
        Error::AddressError(Box::new(source))
    }
}

impl From<RenderError> for Error {
    fn from(source: RenderError) -> Error {
        Error::HandlebarsRenderError(Box::new(source))
    }
}

impl From<TemplateError> for Error {
    fn from(source: TemplateError) -> Error {
        Error::HandlebarsTemplateError(Box::new(source))
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
            Error::Forbidden(message) => ErrorResponse {
                status: 401,
                message: message.clone(),
                context: None,
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
            Error::CryptoError(message) => ErrorResponse {
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
            Error::JWTError(message) => ErrorResponse {
                status: 401,
                message: message.to_string(),
                context: None,
            },
            Error::StorageError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::ReqwestError(error) => ErrorResponse {
                status: error.status().map(|e| e.as_u16()).unwrap_or(500),
                message: "ReqwestError: Downstream error".to_string(),
                context: Some(serde_json::Value::String(error.to_string())),
            },
            Error::MultipartError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::SerdeJsonError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::UuidError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::RustlsError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::RcgenError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::LettreError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::SmtpError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::AddressError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::HandlebarsRenderError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::HandlebarsTemplateError(message) => ErrorResponse {
                status: 500,
                message: message.to_string(),
                context: None,
            },
            Error::TooManyRequests(message) => ErrorResponse {
                status: 429,
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
