use thiserror::Error;

#[derive(Error, Debug)]
#[error("Compatible version is not defined for protocol version {current_protocol_version}")]
pub struct CompatibleProtocolVersionIsNotDefinedError {
    current_protocol_version: u32,
}

impl CompatibleProtocolVersionIsNotDefinedError {
    pub fn new(current_protocol_version: u32) -> Self {
        Self {
            current_protocol_version,
        }
    }

    pub fn current_protocol_version(&self) -> u32 {
        self.current_protocol_version
    }
}
