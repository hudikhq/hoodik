use sea_orm::error::{ColumnFromStrErr, DbErr, RuntimeErr};
use thiserror::Error as ThisError;
use validr::error::ValidationErrors;

pub type AppResult<T> = Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
    NotFound(String),
    DbErr(DbErr),
    RuntimeErr(RuntimeErr),
    ColumnFromStrErr(ColumnFromStrErr),
    Validation(ValidationErrors),
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
