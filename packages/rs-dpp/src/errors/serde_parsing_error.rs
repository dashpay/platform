use serde_json::Error;
use thiserror::Error;

use crate::InvalidVectorSizeError;

#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("Serde parsing error: {message:?}")]
pub struct SerdeParsingError {
    message: String,
}

impl SerdeParsingError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<serde_json::Error> for SerdeParsingError {
    fn from(err: Error) -> Self {
        Self::new(err.to_string())
    }
}

impl From<anyhow::Error> for SerdeParsingError {
    fn from(err: anyhow::Error) -> Self {
        Self::new(err.to_string())
    }
}

impl From<InvalidVectorSizeError> for SerdeParsingError {
    fn from(err: InvalidVectorSizeError) -> Self {
        Self::new(err.to_string())
    }
}

impl From<String> for SerdeParsingError {
    fn from(string: String) -> Self {
        Self::new(string)
    }
}
