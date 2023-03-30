use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use thiserror::Error;

// TODO wrong params - getValidationError missing
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid instant lock proof: ${message}")]
pub struct InvalidInstantAssetLockProofError {
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
