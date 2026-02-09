use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Withdrawal balance mismatch: inputs={input_sum} must equal output={output_sum} + withdrawal={withdrawal_amount}")]
#[platform_serialize(unversioned)]
pub struct WithdrawalBalanceMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    input_sum: u64,
    output_sum: u64,
    withdrawal_amount: u64,
}

impl WithdrawalBalanceMismatchError {
    pub fn new(input_sum: u64, output_sum: u64, withdrawal_amount: u64) -> Self {
        Self {
            input_sum,
            output_sum,
            withdrawal_amount,
        }
    }

    pub fn input_sum(&self) -> u64 {
        self.input_sum
    }

    pub fn output_sum(&self) -> u64 {
        self.output_sum
    }

    pub fn withdrawal_amount(&self) -> u64 {
        self.withdrawal_amount
    }
}

impl From<WithdrawalBalanceMismatchError> for ConsensusError {
    fn from(err: WithdrawalBalanceMismatchError) -> Self {
        Self::BasicError(BasicError::WithdrawalBalanceMismatchError(err))
    }
}
