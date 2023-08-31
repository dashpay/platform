use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Credit withdrawal amount {amount} must be greater or equal to {min_amount}")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityCreditWithdrawalTransitionAmountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub amount: u64,
    pub min_amount: u64,
}

impl InvalidIdentityCreditWithdrawalTransitionAmountError {
    pub fn new(amount: u64, min_amount: u64) -> Self {
        Self { amount, min_amount }
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn min_amount(&self) -> u64 {
        self.min_amount
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionAmountError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionAmountError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(err))
    }
}
