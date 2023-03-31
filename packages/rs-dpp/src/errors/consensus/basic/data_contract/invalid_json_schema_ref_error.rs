use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

// TODO rename message and error
// TODO missed setRefError
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Invalid JSON Schema $ref: {error_message}")]
pub struct InvalidJsonSchemaRefError {
    error_message: String,
}

impl InvalidJsonSchemaRefError {
    pub fn new(error_message: String) -> Self {
        Self { error_message }
    }

    pub fn error_message(&self) -> String {
        self.error_message.clone()
    }
}

impl From<InvalidJsonSchemaRefError> for ConsensusError {
    fn from(err: InvalidJsonSchemaRefError) -> Self {
        Self::BasicError(BasicError::InvalidJsonSchemaRefError(err))
    }
}
