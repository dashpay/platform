use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Encode, Decode)]
#[error("Invalid instant lock proof signature")]
pub struct InvalidInstantAssetLockProofSignatureError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl InvalidInstantAssetLockProofSignatureError {
    pub fn new() -> Self {
        Self
    }
}
impl From<InvalidInstantAssetLockProofSignatureError> for ConsensusError {
    fn from(err: InvalidInstantAssetLockProofSignatureError) -> Self {
        Self::BasicError(BasicError::InvalidInstantAssetLockProofSignatureError(err))
    }
}
