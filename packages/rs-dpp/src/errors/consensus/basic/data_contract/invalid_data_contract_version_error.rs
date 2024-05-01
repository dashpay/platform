use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract version must be {expected_version}, got {version}")]
#[platform_serialize(unversioned)]
pub struct InvalidDataContractVersionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    expected_version: u32,
    version: u32,
}

impl InvalidDataContractVersionError {
    pub fn new(expected_version: u32, version: u32) -> Self {
        Self {
            expected_version,
            version,
        }
    }

    pub fn expected_version(&self) -> u32 {
        self.expected_version
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

impl From<InvalidDataContractVersionError> for ConsensusError {
    fn from(err: InvalidDataContractVersionError) -> Self {
        Self::BasicError(BasicError::InvalidDataContractVersionError(err))
    }
}
