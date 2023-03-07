use thiserror::Error;

use crate::prelude::ProtocolVersion;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Latest supported version is {latest_version:?}")]
pub struct UnsupportedProtocolVersionError {
    parsed_protocol_version: ProtocolVersion,
    latest_version: ProtocolVersion,
}

impl UnsupportedProtocolVersionError {
    pub fn new(parsed_protocol_version: ProtocolVersion, latest_version: ProtocolVersion) -> Self {
        Self {
            parsed_protocol_version,
            latest_version,
        }
    }

    pub fn parsed_protocol_version(&self) -> ProtocolVersion {
        self.parsed_protocol_version
    }

    pub fn latest_version(&self) -> ProtocolVersion {
        self.latest_version
    }
}
