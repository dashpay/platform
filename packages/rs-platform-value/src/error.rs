use std::error;
use std::fmt::Display;
use thiserror::Error;

#[derive(Error, Clone, Eq, PartialEq, Debug)]
pub enum Error {
    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("structure error: {0}")]
    StructureError(String),

    #[error("path error: {0}")]
    PathError(String),

    #[error("integer out of bounds")]
    IntegerSizeError,

    #[error("string decoding error {0}")]
    StringDecodingError(String),

    #[error("key must be a string")]
    KeyMustBeAString,

    #[error("byte length not 32 bytes error")]
    ByteLengthNot32BytesError,
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self where T: Display {
        todo!()
    }
}

