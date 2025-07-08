use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::group::GroupMemberPower;
use crate::errors::ProtocolError;
use crate::identifier::Identifier;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Member {member_id} has a power of {power}, which exceeds the allowed limit of {max_power}"
)]
#[platform_serialize(unversioned)]
pub struct GroupMemberHasPowerOverLimitError {
    member_id: Identifier,
    power: GroupMemberPower,
    max_power: GroupMemberPower,
}

impl GroupMemberHasPowerOverLimitError {
    /// Creates a new `GroupMemberHasPowerOverLimitError`.
    pub fn new(
        member_id: Identifier,
        power: GroupMemberPower,
        max_power: GroupMemberPower,
    ) -> Self {
        Self {
            member_id,
            power,
            max_power,
        }
    }

    /// Returns the identifier of the member.
    pub fn member_id(&self) -> Identifier {
        self.member_id
    }

    /// Returns the power of the member.
    pub fn power(&self) -> GroupMemberPower {
        self.power
    }

    /// Returns the maximum allowed power.
    pub fn max_power(&self) -> GroupMemberPower {
        self.max_power
    }
}

impl From<GroupMemberHasPowerOverLimitError> for ConsensusError {
    fn from(err: GroupMemberHasPowerOverLimitError) -> Self {
        Self::BasicError(BasicError::GroupMemberHasPowerOverLimitError(err))
    }
}
