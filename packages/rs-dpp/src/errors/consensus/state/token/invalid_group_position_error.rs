use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::GroupContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Invalid group position: {invalid_group_position}. {max_group_message}",
    max_group_message = if let Some(max) = self.max_group_position {
        format!("The maximum allowed group position is {}", max)
    } else {
        "No maximum group position limit is set.".to_string()
    }
)]
#[platform_serialize(unversioned)]
pub struct InvalidGroupPositionError {
    max_group_position: Option<GroupContractPosition>,
    invalid_group_position: GroupContractPosition,
}

impl InvalidGroupPositionError {
    pub fn new(
        max_group_position: Option<GroupContractPosition>,
        invalid_group_position: GroupContractPosition,
    ) -> Self {
        Self {
            max_group_position,
            invalid_group_position,
        }
    }

    pub fn max_group_position(&self) -> Option<GroupContractPosition> {
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
