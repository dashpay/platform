use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Unrecognized storage key requirements: allowed {:?}, got {}",
    allowed_values,
    received
)]
#[platform_serialize(unversioned)]
pub struct UnknownStorageKeyRequirementsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    allowed_values: Vec<u8>,
    received: i128,
}

impl UnknownStorageKeyRequirementsError {
    pub fn new(allowed_values: Vec<u8>, received: i128) -> Self {
        Self {
            allowed_values,
            received,
        }
    }

    pub fn allowed_values(&self) -> Vec<u8> {
        self.allowed_values.clone()
    }
    pub fn received(&self) -> i128 {
        self.received
    }
}

impl From<UnknownStorageKeyRequirementsError> for ConsensusError {
    fn from(err: UnknownStorageKeyRequirementsError) -> Self {
        Self::BasicError(BasicError::UnknownStorageKeyRequirementsError(err))
    }
}
