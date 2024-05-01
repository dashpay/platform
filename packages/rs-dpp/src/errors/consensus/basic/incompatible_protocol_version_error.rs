use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Protocol version {parsed_protocol_version:?} is not supported. Minimal supported protocol version is {minimal_protocol_version:?}")]
#[platform_serialize(unversioned)]
pub struct IncompatibleProtocolVersionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    parsed_protocol_version: u32,
    minimal_protocol_version: u32,
}

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
impl From<IncompatibleProtocolVersionError> for ConsensusError {
    fn from(err: IncompatibleProtocolVersionError) -> Self {
        Self::BasicError(BasicError::IncompatibleProtocolVersionError(err))
    }
}
