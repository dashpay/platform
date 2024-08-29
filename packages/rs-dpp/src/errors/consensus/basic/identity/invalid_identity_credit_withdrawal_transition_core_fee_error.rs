use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Core fee per byte {core_fee_per_byte:?} must be part of fibonacci sequence and not less than {min_core_fee_per_byte:?}")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    core_fee_per_byte: u32,
    min_core_fee_per_byte: u32,
}

impl InvalidIdentityCreditWithdrawalTransitionCoreFeeError {
    pub fn new(core_fee_per_byte: u32, min_core_fee_per_byte: u32) -> Self {
        Self {
            core_fee_per_byte,
            min_core_fee_per_byte,
        }
    }

    pub fn core_fee_per_byte(&self) -> u32 {
        self.core_fee_per_byte
    }
    pub fn min_core_fee_per_byte(&self) -> u32 {
        self.min_core_fee_per_byte
    }
}

impl From<InvalidIdentityCreditWithdrawalTransitionCoreFeeError> for ConsensusError {
    fn from(err: InvalidIdentityCreditWithdrawalTransitionCoreFeeError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(err))
    }
}
