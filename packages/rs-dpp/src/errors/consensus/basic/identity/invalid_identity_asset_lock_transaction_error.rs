use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use dashcore;
use dashcore::consensus::encode::Error;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// TODO not primitive
#[derive(Error, Debug, Serialize, Deserialize)]
#[error("Invalid asset lock transaction: ${message}")]
pub struct InvalidIdentityAssetLockTransactionError {
    message: String,
}

impl InvalidIdentityAssetLockTransactionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<InvalidIdentityAssetLockTransactionError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityAssetLockTransactionError(err))
    }
}
