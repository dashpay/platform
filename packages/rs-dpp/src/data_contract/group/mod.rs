use crate::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use crate::data_contract::group::v0::GroupV0;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod accessors;
mod v0;

pub type GroupName = String;
pub type RequiredSigners = u8;

pub type GroupMemberPower = u32;
pub type GroupRequiredPower = u32;
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum Group {
    V0(GroupV0),
}

impl GroupV0Getters for Group {
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
