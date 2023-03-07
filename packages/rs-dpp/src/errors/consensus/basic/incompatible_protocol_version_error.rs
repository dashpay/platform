use thiserror::Error;

use crate::prelude::ProtocolVersion;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Minimal supported protocol version is {minimal_protocol_version:?}")]
pub struct IncompatibleProtocolVersionError {
    parsed_protocol_version: ProtocolVersion,
    minimal_protocol_version: ProtocolVersion,
}

impl IncompatibleProtocolVersionError {
    pub fn new(
        parsed_protocol_version: ProtocolVersion,
        minimal_protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            parsed_protocol_version,
            minimal_protocol_version,
        }
    }

    pub fn parsed_protocol_version(&self) -> ProtocolVersion {
        self.parsed_protocol_version
    }

    pub fn minimal_protocol_version(&self) -> ProtocolVersion {
        self.minimal_protocol_version
    }
}
