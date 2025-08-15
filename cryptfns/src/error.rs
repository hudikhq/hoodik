use ascon_aead::Error as AsconError;
use base64::DecodeError;
use getrandom::Error as RandomError;
use hex::FromHexError;
use rsa::{
    errors::Error as RSAError, pkcs1::Error as PKCS1Error, pkcs8::spki::Error as SpkiError,
    pkcs8::Error as PKCS8Error, signature::Error as SignatureError,
};
use std::string::FromUtf8Error;
use tokenizers::Error as TokenizersError;

pub type CryptoResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    RSAError(RSAError),
    PKCS1Error(PKCS1Error),
    SpkiError(SpkiError),
    PKCS8Error(PKCS8Error),
    SignatureError(SignatureError),
    FromUtf8Error(FromUtf8Error),
    FromHexError(FromHexError),
    DecodeError(DecodeError),
    AsconError(AsconError),
    RandomError(RandomError),
    TokenizersError(TokenizersError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CryptoError: {self:?}")
    }
}

impl From<RSAError> for Error {
    fn from(error: RSAError) -> Self {
        Error::RSAError(error)
    }
}

impl From<PKCS1Error> for Error {
    fn from(error: PKCS1Error) -> Self {
        Error::PKCS1Error(error)
    }
}

impl From<SpkiError> for Error {
    fn from(error: SpkiError) -> Self {
        Error::SpkiError(error)
    }
}

impl From<PKCS8Error> for Error {
    fn from(error: PKCS8Error) -> Self {
        Error::PKCS8Error(error)
    }
}

impl From<SignatureError> for Error {
    fn from(error: SignatureError) -> Self {
        Error::SignatureError(error)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Error::FromUtf8Error(error)
    }
}

impl From<FromHexError> for Error {
    fn from(error: FromHexError) -> Self {
        Error::FromHexError(error)
    }
}

impl From<DecodeError> for Error {
    fn from(error: DecodeError) -> Self {
        Error::DecodeError(error)
    }
}

impl From<AsconError> for Error {
    fn from(error: AsconError) -> Self {
        Error::AsconError(error)
    }
}

impl From<RandomError> for Error {
    fn from(error: RandomError) -> Self {
        Error::RandomError(error)
    }
}

impl From<TokenizersError> for Error {
    fn from(error: TokenizersError) -> Self {
        Error::TokenizersError(error)
    }
}
