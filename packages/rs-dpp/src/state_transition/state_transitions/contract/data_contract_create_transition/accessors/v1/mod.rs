use crate::contract_group::{ContractGroupMembership, ContractGroupRegistration};
use platform_value::Identifier;

/// Contract group accessors of a data contract create transition. Version 0 transitions carry no
/// contract group data, so they report no registration and no memberships.
pub trait DataContractCreateTransitionAccessorsV1 {
    /// The contract group this transition registers, if any.
    fn contract_group(&self) -> Option<&ContractGroupRegistration>;

    /// The id of the contract group this transition registers, derived from the owner id and
    /// the identity nonce. `None` when the transition registers no group.
    fn contract_group_id(&self) -> Option<Identifier>;

    /// The contract group memberships the created contract declares.
    fn contract_group_memberships(&self) -> &[ContractGroupMembership];
}
