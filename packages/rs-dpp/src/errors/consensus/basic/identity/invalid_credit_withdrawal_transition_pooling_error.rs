use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("pooling {pooling:?} should be equal to 0")]
pub struct InvalidIdentityCreditWithdrawalTransitionPoolingError {
    pooling: u8,
}

impl InvalidIdentityCreditWithdrawalTransitionPoolingError {
    pub fn new(pooling: u8) -> Self {
        Self { pooling }
    }

    pub fn pooling(&self) -> u8 {
        self.pooling
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionPoolingError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionPoolingError) -> Self {
        Self::InvalidIdentityCreditWithdrawalTransitionPoolingError(err)
    }
}
