use thiserror::Error;
use crate::{CompatibleProtocolVersionIsNotDefinedError, SerdeParsingError};

#[derive(Debug, Error)]
pub enum NonConsensusError {
    #[error("Unexpected serde parsing error")]
    SerdeParsingError(SerdeParsingError),
    #[error("{0}")]
    CompatibleProtocolVersionIsNotDefinedError(CompatibleProtocolVersionIsNotDefinedError),
}

impl From<SerdeParsingError> for NonConsensusError {
    fn from(err: SerdeParsingError) -> Self {
        Self::SerdeParsingError(err)
    }
}

impl From<CompatibleProtocolVersionIsNotDefinedError> for NonConsensusError {
    fn from(err: CompatibleProtocolVersionIsNotDefinedError) -> Self {
        Self::CompatibleProtocolVersionIsNotDefinedError(err)
    }
}