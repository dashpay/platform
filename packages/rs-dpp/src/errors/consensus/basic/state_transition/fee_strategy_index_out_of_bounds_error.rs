use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Fee strategy {strategy_type} index {index} is out of bounds (count: {count})")]
#[platform_serialize(unversioned)]
pub struct FeeStrategyIndexOutOfBoundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    strategy_type: String,
    index: u16,
    count: u16,
}

impl FeeStrategyIndexOutOfBoundsError {
    pub fn new(strategy_type: impl Into<String>, index: u16, count: u16) -> Self {
        Self {
            strategy_type: strategy_type.into(),
            index,
            count,
        }
    }

    pub fn strategy_type(&self) -> &str {
        &self.strategy_type
    }

    pub fn index(&self) -> u16 {
        self.index
    }

    pub fn count(&self) -> u16 {
        self.count
    }
}

impl From<FeeStrategyIndexOutOfBoundsError> for ConsensusError {
    fn from(err: FeeStrategyIndexOutOfBoundsError) -> Self {
        Self::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(err))
    }
}
