use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::GroupContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

/// Error raised when a group position does not exist in the data contract.
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Group position {} does not exist", missing_group_position)]
#[platform_serialize(unversioned)]
pub struct GroupPositionDoesNotExistError {
    missing_group_position: GroupContractPosition,
}

impl GroupPositionDoesNotExistError {
    /// Creates a new instance of `GroupPositionDoesNotExistError`.
    ///
    /// # Parameters
    /// - `missing_group_position`: The group position that does not exist.
    ///
    /// # Returns
    /// A new `GroupPositionDoesNotExistError` instance.
    pub fn new(missing_group_position: GroupContractPosition) -> Self {
        Self {
            missing_group_position,
        }
    }

    /// Gets the missing group position that caused this error.
    ///
    /// # Returns
    /// The missing group position.
    pub fn missing_group_position(&self) -> GroupContractPosition {
        self.missing_group_position
    }
}

impl From<GroupPositionDoesNotExistError> for ConsensusError {
    fn from(err: GroupPositionDoesNotExistError) -> Self {
        Self::BasicError(BasicError::GroupPositionDoesNotExistError(err))
    }
}
