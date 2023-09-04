use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid instant lock proof: ${message}")]
#[platform_serialize(unversioned)]
pub struct InvalidInstantAssetLockProofError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub message: String,
}

impl InvalidInstantAssetLockProofError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<InvalidInstantAssetLockProofError> for ConsensusError {
    fn from(err: InvalidInstantAssetLockProofError) -> Self {
        Self::BasicError(BasicError::InvalidInstantAssetLockProofError(err))
    }
}
