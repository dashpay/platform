use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use dashcore;
use dashcore::consensus::encode::Error;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
#[error("Invalid asset lock transaction: ${error_message}")]
pub struct InvalidIdentityAssetLockTransactionError {
    error_message: String,
}

impl InvalidIdentityAssetLockTransactionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            error_message: message.into(),
        }
    }

    pub fn error_message(&self) -> &str {
        &self.error_message
    }
}

impl From<InvalidIdentityAssetLockTransactionError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityAssetLockTransactionError(err))
    }
}
