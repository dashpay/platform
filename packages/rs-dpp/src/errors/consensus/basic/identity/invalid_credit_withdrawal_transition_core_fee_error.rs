use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Core fee per byte {core_fee_per_byte:?} must be part of fibonacci sequence")]
pub struct InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    core_fee_per_byte: u32,
}

impl InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    pub fn new(core_fee_per_byte: u32) -> Self {
        Self { core_fee_per_byte }
    }

    pub fn core_fee_per_byte(&self) -> u32 {
        self.core_fee_per_byte
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionCoreFeeError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionCoreFeeError) -> Self {
        Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(err)
    }
}
