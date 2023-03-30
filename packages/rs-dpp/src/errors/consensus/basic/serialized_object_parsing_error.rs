use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("Parsing of serialized object failed due to: {parsing_error}")]
pub struct SerializedObjectParsingError {
    parsing_error: anyhow::Error
}

impl SerializedObjectParsingError {
    pub fn new(parsing_error: anyhow::Error) -> Self {
        Self {
            parsing_error
        }
    }

    pub fn parsing_error(&self) -> &anyhow::Error {
        &self.parsing_error
    }
}
impl From<SerializedObjectParsingError> for ConsensusError {
    fn from(err: SerializedObjectParsingError) -> Self {
        Self::BasicError(BasicError::SerializedObjectParsingError(err))
    }
}
