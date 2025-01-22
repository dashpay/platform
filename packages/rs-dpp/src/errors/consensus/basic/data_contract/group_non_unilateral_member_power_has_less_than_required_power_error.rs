use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::group::GroupMemberPower;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("The collective power of non-unilateral members {total_power} is less than the required power {required_power}")]
#[platform_serialize(unversioned)]
pub struct GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError {
    total_power: GroupMemberPower,
    required_power: GroupMemberPower,
}

impl GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError {
    /// Creates a new `GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError`.
    ///
    /// # Parameters
    /// - `total_power`: The total power of non-unilateral members.
    /// - `required_power`: The required power to meet the threshold.
    pub fn new(total_power: GroupMemberPower, required_power: GroupMemberPower) -> Self {
        Self {
            total_power,
            required_power,
        }
    }

    /// Returns the total power of non-unilateral members.
    pub fn total_power(&self) -> GroupMemberPower {
        self.total_power
    }

    /// Returns the required power to meet the threshold.
    pub fn required_power(&self) -> GroupMemberPower {
        self.required_power
    }
}

impl From<GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError> for ConsensusError {
    fn from(err: GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError) -> Self {
        Self::BasicError(
            BasicError::GroupNonUnilateralMemberPowerHasLessThanRequiredPowerError(err),
        )
    }
}
