use thiserror::Error;
use crate::errors::consensus::AbstractConsensusError;

#[derive(Error, Debug)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Minimal supported protocol version is {minimal_protocol_version:?}")]
pub struct IncompatibleProtocolVersionError {
    parsed_protocol_version: u32,
    minimal_protocol_version: u32,
}

impl AbstractConsensusError for IncompatibleProtocolVersionError {}

impl IncompatibleProtocolVersionError {
    pub fn new(parsed_protocol_version: u32, minimal_protocol_version: u32) -> Self {
        Self {
            parsed_protocol_version,
            minimal_protocol_version,
        }
    }

    pub fn parsed_protocol_version(&self) -> u32 {
        self.parsed_protocol_version
    }

    pub fn minimal_protocol_version(&self) -> u32 {
        self.minimal_protocol_version
    }
}
