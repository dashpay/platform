use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug)]
#[error("Can't read protocol version from serialized object: {parsing_error}")]
pub struct ProtocolVersionParsingError {
    pub parsing_error: anyhow::Error,
}

impl ProtocolVersionParsingError {
    pub fn new(parsing_error: anyhow::Error) -> Self {
        Self { parsing_error }
    }
}

impl From<ProtocolVersionParsingError> for ConsensusError {
    fn from(err: ProtocolVersionParsingError) -> Self {
        Self::ProtocolVersionParsingError(err)
    }
}
