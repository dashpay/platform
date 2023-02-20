use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(
    "pooling {pooling:?} should be equal to 0. Other pooling mechanism are not implemented yet"
)]
pub struct NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
    pooling: u8,
}

impl NotImplementedIdentityCreditWithdrawalTransitionPoolingError {
    pub fn new(pooling: u8) -> Self {
        Self { pooling }
    }

    pub fn pooling(&self) -> u8 {
        self.pooling
    }
}

impl From<NotImplementedIdentityCreditWithdrawalTransitionPoolingError> for ConsensusError {
    fn from(err: NotImplementedIdentityCreditWithdrawalTransitionPoolingError) -> Self {
        Self::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(err)
    }
}
