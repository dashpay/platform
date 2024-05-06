use std::fmt::Display;

use thiserror::Error;

#[derive(Error, Clone, Eq, PartialEq, Debug)]
#[ferment_macro::export]
pub enum Error {
    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("structure error: {0}")]
    StructureError(String),

    #[error("path error: {0}")]
    PathError(String),

    #[error("integer out of bounds")]
    IntegerSizeError,

    #[error("integer parsing")]
    IntegerParsingError,

    #[error("string decoding error {0}")]
    StringDecodingError(String),

    #[error("key must be a string")]
    KeyMustBeAString,

    #[error("byte length not 20 bytes error: {0}")]
    ByteLengthNot20BytesError(String),

    #[error("byte length not 32 bytes error: {0}")]
    ByteLengthNot32BytesError(String),

    #[error("byte length not 36 bytes error: {0}")]
    ByteLengthNot36BytesError(String),

    #[error("serde serialization error: {0}")]
    SerdeSerializationError(String),

    #[error("serde deserialization error: {0}")]
    SerdeDeserializationError(String),

    #[error("cbor serialization error: {0}")]
    CborSerializationError(String),
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::SerdeSerializationError(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::SerdeDeserializationError(msg.to_string())
    }
}
