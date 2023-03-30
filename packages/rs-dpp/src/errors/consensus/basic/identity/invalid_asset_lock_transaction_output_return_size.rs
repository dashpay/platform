use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error(
    "Asset Lock output ${output_index} has invalid public key hash. Must be 20 length bytes hash"
)]
pub struct InvalidAssetLockTransactionOutputReturnSizeError {
    output_index: usize,
}

impl InvalidAssetLockTransactionOutputReturnSizeError {
    pub fn new(output_index: usize) -> Self {
        Self { output_index }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }
}

impl From<InvalidAssetLockTransactionOutputReturnSizeError> for ConsensusError {
    fn from(err: InvalidAssetLockTransactionOutputReturnSizeError) -> Self {
        Self::BasicError(BasicError::InvalidAssetLockTransactionOutputReturnSizeError(err))
    }
}
