use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Invalid instant lock proof: ${message}")]
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
