use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[error("Invalid instant lock proof signature")]
pub struct InvalidInstantAssetLockProofSignatureError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

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
