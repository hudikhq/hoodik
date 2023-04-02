use base64::{engine::general_purpose, Engine};
use error::{AppResult, Error};

/// Encode input to base64 string
pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    general_purpose::STANDARD.encode(input)
}

/// Decode base64 string to bytes
pub fn decode(input: &str) -> AppResult<Vec<u8>> {
    general_purpose::STANDARD.decode(input).map_err(Error::from)
}

#[cfg(test)]
mod test {
    use super::*;
    const FRONTEND_HELLO_WORLD_BASE64: &str = "aGVsbG8gd29ybGQ=";

    #[test]
    fn test_base64_encode() {
        let input = "hello world";
        let output = encode(input);
        assert_eq!(output, FRONTEND_HELLO_WORLD_BASE64);
    }

    #[test]
    fn test_base64_decode() {
        let input = FRONTEND_HELLO_WORLD_BASE64;
        let output = decode(input).unwrap();
        assert_eq!(output, b"hello world");
    }
}
