use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("Can't read protocol version from serialized object: {parsing_error}")]
pub struct ProtocolVersionParsingError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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

impl From<ProtocolVersionParsingError> for u32 {
    fn from(val: ProtocolVersionParsingError) -> Self {
        0
    }
}
