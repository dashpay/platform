use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Invalid {}: {}", identifier_name, error_message)]
pub struct InvalidIdentifierError {
    identifier_name: String,
    error_message: String,
}

impl InvalidIdentifierError {
    pub fn new(identifier_name: String, error: String) -> Self {
        Self {
            identifier_name,
            error_message: error,
        }
    }

    pub fn identifier_name(&self) -> &str {
        &self.identifier_name
    }

    pub fn error_message(&self) -> &str {
        &self.error_message
    }
}

impl From<InvalidIdentifierError> for ConsensusError {
    fn from(err: InvalidIdentifierError) -> Self {
        Self::BasicError(BasicError::InvalidIdentifierError(err))
    }
}
