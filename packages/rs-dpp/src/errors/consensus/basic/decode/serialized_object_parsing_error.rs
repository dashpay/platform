use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("Parsing of serialized object failed due to: {parsing_error}")]
pub struct SerializedObjectParsingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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
