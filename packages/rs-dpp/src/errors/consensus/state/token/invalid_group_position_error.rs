use crate::consensus::ConsensusError;
use crate::data_contract::GroupContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
use crate::consensus::state::state_error::StateError;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid group position {}, max {}",
    invalid_group_position,
    max_group_position
)]
#[platform_serialize(unversioned)]
pub struct InvalidGroupPositionError {
    max_group_position: GroupContractPosition,
    invalid_group_position: GroupContractPosition,
}

impl InvalidGroupPositionError {
    pub fn new(
        max_group_position: GroupContractPosition,
        invalid_group_position: GroupContractPosition,
    ) -> Self {
        Self {
            max_group_position,
            invalid_group_position,
        }
    }

    pub fn max_group_position(&self) -> GroupContractPosition {
        self.max_group_position
    }

    pub fn invalid_group_position(&self) -> GroupContractPosition {
        self.invalid_group_position
    }
}

impl From<InvalidGroupPositionError> for ConsensusError {
    fn from(err: InvalidGroupPositionError) -> Self {
        Self::StateError(StateError::InvalidGroupPositionError(err))
    }
}
