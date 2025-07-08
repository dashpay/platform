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
#[error("Group total power {total_power} is less than the required power {required_power}")]
#[platform_serialize(unversioned)]
pub struct GroupTotalPowerLessThanRequiredError {
    total_power: GroupMemberPower,
    required_power: GroupMemberPower,
}

impl GroupTotalPowerLessThanRequiredError {
    pub fn new(total_power: GroupMemberPower, required_power: GroupMemberPower) -> Self {
        Self {
            total_power,
            required_power,
        }
    }

    pub fn total_power(&self) -> GroupMemberPower {
        self.total_power
    }

    pub fn required_power(&self) -> GroupMemberPower {
        self.required_power
    }
}

impl From<GroupTotalPowerLessThanRequiredError> for ConsensusError {
    fn from(err: GroupTotalPowerLessThanRequiredError) -> Self {
        Self::BasicError(BasicError::GroupTotalPowerLessThanRequiredError(err))
    }
}
