use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// TODO missed setParsingError
#[derive(Error, Debug, Serialize, Deserialize)]
#[error("Parsing of serialized object failed due to: {parsing_error}")]
pub struct SerializedObjectParsingError {
    parsing_error: String,
}

impl SerializedObjectParsingError {
    pub fn new(parsing_error: String) -> Self {
        Self { parsing_error }
    }

    pub fn parsing_error(&self) -> &str {
        &self.parsing_error
    }
}
impl From<SerializedObjectParsingError> for ConsensusError {
    fn from(err: SerializedObjectParsingError) -> Self {
        Self::BasicError(BasicError::SerializedObjectParsingError(err))
    }
}
