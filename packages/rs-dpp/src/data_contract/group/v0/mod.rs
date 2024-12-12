use crate::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
use crate::data_contract::group::{GroupMemberPower, GroupRequiredPower};
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub struct GroupV0 {
    pub members: BTreeMap<Identifier, GroupMemberPower>,
    pub required_power: GroupRequiredPower,
}

impl GroupV0Getters for GroupV0 {
    fn members(&self) -> &BTreeMap<Identifier, u32> {
        &self.members
    }

    fn members_mut(&mut self) -> &mut BTreeMap<Identifier, u32> {
        &mut self.members
    }

    fn required_power(&self) -> GroupRequiredPower {
        self.required_power
    }
}

impl GroupV0Setters for GroupV0 {
    fn set_members(&mut self, members: BTreeMap<Identifier, u32>) {
        self.members = members;
    }

    fn set_member_power(&mut self, member_id: Identifier, power: u32) {
        self.members.insert(member_id, power);
    }

    fn remove_member(&mut self, member_id: &Identifier) -> bool {
        self.members.remove(member_id).is_some()
    }

    fn set_required_power(&mut self, required_power: GroupRequiredPower) {
        self.required_power = required_power;
    }
}
