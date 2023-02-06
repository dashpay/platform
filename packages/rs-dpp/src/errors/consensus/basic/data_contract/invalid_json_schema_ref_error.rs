use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid JSON Schema $ref: {ref_error}")]
pub struct InvalidJsonSchemaRefError {
    ref_error: String,
}

impl InvalidJsonSchemaRefError {
    pub fn new(ref_error: String) -> Self {
        Self { ref_error }
    }

    pub fn ref_error(&self) -> String {
        self.ref_error.clone()
    }
}

impl From<InvalidJsonSchemaRefError> for ConsensusError {
    fn from(err: InvalidJsonSchemaRefError) -> Self {
        Self::InvalidJsonSchemaRefError(err)
    }
}
