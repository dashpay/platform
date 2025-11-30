use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("ReduceWithdrawal must be the last step in fee strategy")]
#[platform_serialize(unversioned)]
pub struct FeeStrategyReduceWithdrawalNotLastError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl FeeStrategyReduceWithdrawalNotLastError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for FeeStrategyReduceWithdrawalNotLastError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<FeeStrategyReduceWithdrawalNotLastError> for ConsensusError {
    fn from(err: FeeStrategyReduceWithdrawalNotLastError) -> Self {
        Self::BasicError(BasicError::FeeStrategyReduceWithdrawalNotLastError(err))
    }
}
