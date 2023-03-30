use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default)]
#[error("Invalid instant lock proof signature")]
pub struct InvalidInstantAssetLockProofSignatureError;

impl InvalidInstantAssetLockProofSignatureError {
    pub fn new() -> Self {
        Self::default()
    }
}
impl From<InvalidInstantAssetLockProofSignatureError> for ConsensusError {
    fn from(err: InvalidInstantAssetLockProofSignatureError) -> Self {
        Self::BasicError(BasicError::InvalidInstantAssetLockProofSignatureError(err))
    }
}
