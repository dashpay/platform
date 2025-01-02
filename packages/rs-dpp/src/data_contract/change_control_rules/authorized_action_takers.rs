use crate::data_contract::group::accessors::v0::GroupV0Getters;
use crate::data_contract::group::{Group, GroupMemberPower};
use crate::multi_identity_events::ActionTaker;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, Default)]
pub enum AuthorizedActionTakers {
    #[default]
    NoOne,
    ContractOwner,
    MainGroup,
    Group(Group),
}

impl AuthorizedActionTakers {
    pub fn allowed_for_action_taker(
        &self,
        contract_owner_id: &Identifier,
        main_group: &Group,
        action_taker: &ActionTaker,
    ) -> bool {
        match self {
            // No one is allowed
            AuthorizedActionTakers::NoOne => false,

            // Only the contract owner is allowed
            AuthorizedActionTakers::ContractOwner => match action_taker {
                ActionTaker::SingleIdentity(action_taker) => action_taker == contract_owner_id,
                ActionTaker::SpecifiedIdentities(action_takers) => {
                    action_takers.contains(contract_owner_id)
                }
            },

            // MainGroup allows multiparty actions with specific power requirements
            AuthorizedActionTakers::MainGroup => {
                Self::is_action_taker_authorized(main_group, action_taker)
            }

            // Group-specific permissions with power aggregation logic
            AuthorizedActionTakers::Group(group) => {
                Self::is_action_taker_authorized(group, action_taker)
            }
        }
    }

    /// Helper method to check if action takers meet the group's required power threshold.
    fn is_action_taker_authorized(group: &Group, action_taker: &ActionTaker) -> bool {
        match action_taker {
            ActionTaker::SingleIdentity(_) => false,
            ActionTaker::SpecifiedIdentities(action_takers) => {
                // Calculate the total power of action takers who are members of the group
                let total_power: GroupMemberPower = group
                    .members()
                    .iter()
                    .filter(|(member_id, _)| action_takers.contains(*member_id))
                    .map(|(_, power)| *power)
                    .sum();

                // Compare total power to the group's required power
                total_power >= group.required_power() as GroupMemberPower
            }
        }
    }
}
