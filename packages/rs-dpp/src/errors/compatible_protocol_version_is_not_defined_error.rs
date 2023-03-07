use thiserror::Error;

use crate::prelude::ProtocolVersion;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Compatible version is not defined for protocol version {current_protocol_version}")]
pub struct CompatibleProtocolVersionIsNotDefinedError {
    current_protocol_version: ProtocolVersion,
}

impl CompatibleProtocolVersionIsNotDefinedError {
    pub fn new(current_protocol_version: ProtocolVersion) -> Self {
        Self {
            current_protocol_version,
        }
    }

    pub fn current_protocol_version(&self) -> u32 {
        self.current_protocol_version
    }
}
