use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use dashcore;
use dashcore::consensus::encode::Error;
use thiserror::Error;

// TODO not primitive
#[derive(Error, Debug)]
#[error("Invalid asset lock transaction: ${message}")]
pub struct InvalidIdentityAssetLockTransactionError {
    message: String,
    validation_error: Option<dashcore::consensus::encode::Error>,
}

impl InvalidIdentityAssetLockTransactionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            validation_error: None,
        }
    }

    pub fn set_validation_error(&mut self, error: dashcore::consensus::encode::Error) {
        self.validation_error = Some(error);
    }

    pub fn validation_error(&self) -> Option<&Error> {
        self.validation_error.as_ref()
    }
}
impl From<InvalidIdentityAssetLockTransactionError> for ConsensusError {
    fn from(err: InvalidIdentityAssetLockTransactionError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityAssetLockTransactionError(err))
    }
}
