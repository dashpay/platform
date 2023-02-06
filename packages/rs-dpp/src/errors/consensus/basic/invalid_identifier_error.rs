use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid {}: {}", identifier_name, error)]
pub struct InvalidIdentifierError {
    identifier_name: String,
    error: String,
}

impl InvalidIdentifierError {
    pub fn new(identifier_name: String, error: String) -> Self {
        Self {
            identifier_name,
            error,
        }
    }

    pub fn identifier_name(&self) -> String {
        self.identifier_name.clone()
    }

    pub fn error(&self) -> String {
        self.error.clone()
    }
}

impl From<InvalidIdentifierError> for BasicError {
    fn from(err: InvalidIdentifierError) -> Self {
        Self::InvalidIdentifierError(err)
    }
}
