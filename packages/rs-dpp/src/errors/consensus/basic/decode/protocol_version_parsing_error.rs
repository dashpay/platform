use crate::consensus::basic::BasicError;
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::ProtocolError;

#[derive(Error, Debug)]
#[error("Can't read protocol version from serialized object: {parsing_error}")]
pub struct ProtocolVersionParsingError {
    pub parsing_error: ProtocolError,
}

impl ProtocolVersionParsingError {
    pub fn new(parsing_error: ProtocolError) -> Self {
        Self { parsing_error }
    }
}

impl From<ProtocolVersionParsingError> for ConsensusError {
    fn from(err: ProtocolVersionParsingError) -> Self {
        Self::BasicError(BasicError::ProtocolVersionParsingError(err))
    }
}
