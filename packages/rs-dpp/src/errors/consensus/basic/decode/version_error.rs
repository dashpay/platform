use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Can't read protocol version from serialized object: {error}")]
#[platform_serialize(unversioned)]
pub struct VersionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    error: String,
}

impl VersionError {
    pub fn new(error: String) -> Self {
        Self { error }
    }

    pub fn error(&self) -> &str {
        &self.error
    }
}

impl From<VersionError> for ConsensusError {
    fn from(err: VersionError) -> Self {
        Self::BasicError(BasicError::VersionError(err))
    }
}

impl From<VersionError> for u32 {
    fn from(_val: VersionError) -> Self {
        0
    }
}

impl From<&str> for VersionError {
    fn from(value: &str) -> Self {
        VersionError::new(value.to_string())
    }
}

impl From<String> for VersionError {
    fn from(value: String) -> Self {
        VersionError::new(value)
    }
}
