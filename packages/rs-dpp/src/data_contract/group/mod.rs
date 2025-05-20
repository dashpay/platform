use crate::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use crate::data_contract::group::methods::v0::GroupMethodsV0;
use crate::data_contract::group::v0::GroupV0;
use crate::errors::ProtocolError;
use crate::validation::SimpleConsensusValidationResult;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod accessors;
pub(crate) mod methods;
pub mod v0;

pub type RequiredSigners = u8;

#[cfg_attr(feature = "apple", ferment_macro::export)]
pub type GroupMemberPower = u32;
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub type GroupSumPower = u32;
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub type GroupRequiredPower = u32;
#[derive(
    Serialize,
    Deserialize,
    Decode,
    Encode,
    PlatformSerialize,
    PlatformDeserialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum Group {
    V0(GroupV0),
}

impl GroupV0Getters for Group {
    fn member_power(&self, member_id: Identifier) -> Result<u32, ProtocolError> {
        match self {
            Group::V0(group_v0) => group_v0.member_power(member_id),
        }
    }
    fn members(&self) -> &BTreeMap<Identifier, u32> {
        match self {
            Group::V0(group_v0) => group_v0.members(),
        }
    }

    fn members_mut(&mut self) -> &mut BTreeMap<Identifier, u32> {
        match self {
            Group::V0(group_v0) => group_v0.members_mut(),
        }
    }

    fn required_power(&self) -> GroupRequiredPower {
        match self {
            Group::V0(group_v0) => group_v0.required_power(),
        }
    }
}

impl GroupV0Setters for Group {
    fn set_members(&mut self, members: BTreeMap<Identifier, u32>) {
        match self {
            Group::V0(group_v0) => group_v0.set_members(members),
        }
    }

    fn set_member_power(&mut self, member_id: Identifier, power: u32) {
        match self {
            Group::V0(group_v0) => group_v0.set_member_power(member_id, power),
        }
    }

    fn remove_member(&mut self, member_id: &Identifier) -> bool {
        match self {
            Group::V0(group_v0) => group_v0.remove_member(member_id),
        }
    }

    fn set_required_power(&mut self, required_power: GroupRequiredPower) {
        match self {
            Group::V0(group_v0) => group_v0.set_required_power(required_power),
        }
    }
}

impl GroupMethodsV0 for Group {
    fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match self {
            Group::V0(group_v0) => group_v0.validate(platform_version),
        }
    }
}
