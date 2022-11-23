use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Core fee {core_fee:?} must be part of fibonacci sequence")]
pub struct InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    core_fee: u32,
}

impl InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    pub fn new(core_fee: u32) -> Self {
        Self { core_fee }
    }

    pub fn core_fee(&self) -> u32 {
        self.core_fee
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionCoreFeeError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionCoreFeeError) -> Self {
        Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(err)
    }
}
