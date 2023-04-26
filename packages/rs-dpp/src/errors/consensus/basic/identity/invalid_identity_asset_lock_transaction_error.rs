use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("Invalid asset lock transaction: ${message}")]
pub struct InvalidIdentityAssetLockTransactionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    message: String,
}

impl InvalidIdentityAssetLockTransactionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<InvalidIdentityAssetLockTransactionError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityAssetLockTransactionError(err))
    }
}
