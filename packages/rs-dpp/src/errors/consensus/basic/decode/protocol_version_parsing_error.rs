use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

// TODO not primitive
#[derive(Error, Debug, Serialize, Deserialize)]
#[error("Can't read protocol version from serialized object: {parsing_error}")]
pub struct ProtocolVersionParsingError {
    parsing_error: String,
}

impl ProtocolVersionParsingError {
    pub fn new(parsing_error: String) -> Self {
        Self { parsing_error }
    }

    pub fn parsing_error(&self) -> &str {
        &self.parsing_error
    }
}

impl From<ProtocolVersionParsingError> for ConsensusError {
    fn from(err: ProtocolVersionParsingError) -> Self {
        Self::BasicError(BasicError::ProtocolVersionParsingError(err))
    }
}

impl Into<u32> for ProtocolVersionParsingError {
    fn into(self) -> u32 {
        0
    }
}
