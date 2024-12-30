use crate::data_contract::group::GroupRequiredPower;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::BTreeMap;

/// Getters for GroupV0
pub trait GroupV0Getters {
    /// Returns the member power
    fn member_power(&self, member_id: Identifier) -> Result<u32, ProtocolError>;
    /// Returns the members map of the group
    fn members(&self) -> &BTreeMap<Identifier, u32>;

    /// Returns a mutable reference to the members map of the group
    fn members_mut(&mut self) -> &mut BTreeMap<Identifier, u32>;

    /// Returns the required power of the group
    fn required_power(&self) -> GroupRequiredPower;
}

/// Setters for GroupV0
pub trait GroupV0Setters {
    /// Sets the members of the group
    fn set_members(&mut self, members: BTreeMap<Identifier, u32>);

    /// Inserts or updates a member with a specific power
    fn set_member_power(&mut self, member_id: Identifier, power: u32);

    /// Removes a member from the group
    fn remove_member(&mut self, member_id: &Identifier) -> bool;

    /// Sets the required power of the group
    fn set_required_power(&mut self, required_power: GroupRequiredPower);
}
