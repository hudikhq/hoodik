use std::fmt;

#[derive(Debug)]
pub enum Error {
    Crypto(String),
    Http(HttpError),
    Io(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct HttpError {
    pub status: u16,
    pub message: String,
    /// Validation errors returned by the backend (e.g. `{"checksum": "mismatch"}`).
    pub validation: Option<std::collections::HashMap<String, String>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Crypto(msg) => write!(f, "Crypto error: {msg}"),
            Error::Http(e) => write!(f, "HTTP {}: {}", e.status, e.message),
            Error::Io(msg) => write!(f, "IO error: {msg}"),
            Error::Cancelled => write!(f, "Transfer cancelled"),
        }
    }
}

impl From<cryptfns::error::Error> for Error {
    fn from(e: cryptfns::error::Error) -> Self {
        Error::Crypto(format!("{e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
