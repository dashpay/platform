use thiserror::Error;

use crate::{consensus::ConsensusError, prelude::Fee};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Core fee per byte {core_fee_per_byte:?} must be part of fibonacci sequence")]
pub struct InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    core_fee_per_byte: Fee,
}

impl InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    pub fn new(core_fee_per_byte: Fee) -> Self {
        Self { core_fee_per_byte }
    }

    pub fn core_fee_per_byte(&self) -> Fee {
        self.core_fee_per_byte
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionCoreFeeError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionCoreFeeError) -> Self {
        Self::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(err)
    }
}
