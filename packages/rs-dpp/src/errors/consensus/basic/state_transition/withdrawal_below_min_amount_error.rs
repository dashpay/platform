use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Withdrawal amount {withdrawal_amount} must be at least {min_amount} and at most {max_amount}"
)]
#[platform_serialize(unversioned)]
pub struct WithdrawalBelowMinAmountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    withdrawal_amount: u64,
    min_amount: u64,
    max_amount: u64,
}

impl WithdrawalBelowMinAmountError {
    pub fn new(withdrawal_amount: u64, min_amount: u64, max_amount: u64) -> Self {
        Self {
            withdrawal_amount,
            min_amount,
            max_amount,
        }
    }

    pub fn withdrawal_amount(&self) -> u64 {
        self.withdrawal_amount
    }

    pub fn min_amount(&self) -> u64 {
        self.min_amount
    }

    pub fn max_amount(&self) -> u64 {
        self.max_amount
    }
}

impl From<WithdrawalBelowMinAmountError> for ConsensusError {
    fn from(err: WithdrawalBelowMinAmountError) -> Self {
        Self::BasicError(BasicError::WithdrawalBelowMinAmountError(err))
    }
}
