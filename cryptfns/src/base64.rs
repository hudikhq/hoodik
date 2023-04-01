use base64::{engine::general_purpose::STANDARD, Engine};
use error::{AppResult, Error};

/// Encode input to base64 string
pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    STANDARD.encode(input)
}

/// Decode base64 string to bytes
pub fn decode(input: &str) -> AppResult<Vec<u8>> {
    STANDARD.decode(input).map_err(Error::from)
}
