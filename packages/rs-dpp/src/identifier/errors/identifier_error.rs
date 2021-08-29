use crate::util::string_encoding::StringDecodeError;
use std::error::Error;
use std::fmt::{Debug, Formatter, Display};

pub struct IdentifierError {
    pub message: String
}

impl Debug for IdentifierError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Display for IdentifierError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for IdentifierError {}

impl From<StringDecodeError> for IdentifierError {
    fn from(string_decode_error: StringDecodeError) -> Self {
        match string_decode_error {
            StringDecodeError::Base64(decode_error) => {
                IdentifierError { message: decode_error.to_string() }
            }
            StringDecodeError::Base58(decode_error) => {
                IdentifierError { message: decode_error.to_string() }
            }
        }
    }
}