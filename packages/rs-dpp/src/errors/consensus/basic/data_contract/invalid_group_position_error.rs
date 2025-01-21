use crate::consensus::basic::BasicError;
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
    "Invalid group position {}, expected {}",
    invalid_group_position,
    expected_group_position
)]
#[platform_serialize(unversioned)]
pub struct InvalidGroupPositionError {
    expected_group_position: GroupContractPosition,
    invalid_group_position: GroupContractPosition,
}

impl InvalidGroupPositionError {
    pub fn new(
        expected_group_position: GroupContractPosition,
        invalid_group_position: GroupContractPosition,
    ) -> Self {
        Self {
            expected_group_position,
            invalid_group_position,
        }
    }

    pub fn expected_group_position(&self) -> GroupContractPosition {
        self.expected_group_position
    }

    pub fn invalid_group_position(&self) -> GroupContractPosition {
        self.invalid_group_position
    }
}

impl From<InvalidGroupPositionError> for ConsensusError {
    fn from(err: InvalidGroupPositionError) -> Self {
        Self::BasicError(BasicError::InvalidGroupPositionError(err))
    }
}
