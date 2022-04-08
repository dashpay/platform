use thiserror::Error;
use crate::errors::consensus::AbstractConsensusError;

#[derive(Error, Debug, Clone)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Latest supported version is {latest_version:?}")]
pub struct UnsupportedProtocolVersionError {
    parsed_protocol_version: u32,
    latest_version: u32,
}

impl AbstractConsensusError for UnsupportedProtocolVersionError {}

impl UnsupportedProtocolVersionError {
    pub fn new(parsed_protocol_version: u32, latest_version: u32) -> Self {
        Self {
            parsed_protocol_version,
            latest_version,
        }
    }

    pub fn parsed_protocol_version(&self) -> u32 {
        self.parsed_protocol_version
    }

    pub fn latest_version(&self) -> u32 {
        self.latest_version
    }
}
