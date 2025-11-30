use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Fee strategy has {actual_steps} steps, maximum allowed is {max_steps}")]
#[platform_serialize(unversioned)]
pub struct FeeStrategyTooManyStepsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_steps: u8,
    max_steps: u8,
}

impl FeeStrategyTooManyStepsError {
    pub fn new(actual_steps: u8, max_steps: u8) -> Self {
        Self {
            actual_steps,
            max_steps,
        }
    }

    pub fn actual_steps(&self) -> u8 {
        self.actual_steps
    }

    pub fn max_steps(&self) -> u8 {
        self.max_steps
    }
}

impl From<FeeStrategyTooManyStepsError> for ConsensusError {
    fn from(err: FeeStrategyTooManyStepsError) -> Self {
        Self::BasicError(BasicError::FeeStrategyTooManyStepsError(err))
    }
}
