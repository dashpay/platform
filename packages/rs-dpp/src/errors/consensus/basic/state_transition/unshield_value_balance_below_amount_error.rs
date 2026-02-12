use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("value_balance ({value_balance}) must be >= amount ({amount})")]
#[platform_serialize(unversioned)]
pub struct UnshieldValueBalanceBelowAmountError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    value_balance: i64,
    amount: u64,
}

impl UnshieldValueBalanceBelowAmountError {
    pub fn new(value_balance: i64, amount: u64) -> Self {
        Self {
            value_balance,
            amount,
        }
    }

    pub fn value_balance(&self) -> i64 {
        self.value_balance
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl From<UnshieldValueBalanceBelowAmountError> for ConsensusError {
    fn from(err: UnshieldValueBalanceBelowAmountError) -> Self {
        Self::BasicError(BasicError::UnshieldValueBalanceBelowAmountError(err))
    }
}
