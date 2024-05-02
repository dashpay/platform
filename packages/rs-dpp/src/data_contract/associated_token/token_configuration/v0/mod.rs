use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};
use platform_value::Identifier;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::multi_identity_events::ActionTaker;

pub type RequiredSigners = u8;
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum AuthorizedActionTakers {
    NoOne,
    ContractOwner,
    MainGroup,
    SpecifiedIdentities(BTreeSet<Identifier>, RequiredSigners)
}

impl AuthorizedActionTakers {
    pub fn matches_action_taker(&self, contract_owner_id: &Identifier, main_group: &(BTreeSet<Identifier>, RequiredSigners), action_taker: &ActionTaker) -> bool {
        match self {
            AuthorizedActionTakers::NoOne => false,
            AuthorizedActionTakers::ContractOwner => {
                match action_taker {
                    ActionTaker::SingleIdentity(action_taker) => action_taker == contract_owner_id,
                    ActionTaker::SpecifiedIdentities(action_takers) => action_takers.contains(contract_owner_id),
                }
            }
            AuthorizedActionTakers::MainGroup => {
                match action_taker {
                    ActionTaker::SingleIdentity(_) => false,
                    ActionTaker::SpecifiedIdentities(action_takers) => {
                        let authorized_action_takers_count = main_group.0.intersection(action_takers).count();
                        if authorized_action_takers_count > 255 {
                            return false;
                        }
                        authorized_action_takers_count as u8 >= main_group.1
                    },
                }
            }
            AuthorizedActionTakers::SpecifiedIdentities(specified_authorized_identities, required_signers_count) => {
                match action_taker {
                    ActionTaker::SingleIdentity(_) => false,
                    ActionTaker::SpecifiedIdentities(action_takers) => {
                        let authorized_action_takers_count = specified_authorized_identities.intersection(action_takers).count();
                        if authorized_action_takers_count > 255 {
                            return false;
                        }
                        authorized_action_takers_count as u8 >= *required_signers_count
                    },
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub struct ChangeControlRules {
    /// This is who is authorized to make such a change
    authorized_to_make_change: AuthorizedActionTakers,
    /// This is who is authorized to make such a change to the people authorized to make a change
    authorized_to_change_to_authorized_action_takers: AuthorizedActionTakers,
    /// Are we allowed to change to None in the future
    changing_authorized_action_takers_to_no_one_allowed: bool,
    /// Are we allowed to change to None in the future
    changing_authorized_action_takers_to_contract_owner_allowed: bool,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV0 {
    pub max_supply: u64,
    pub max_supply_can_be_increased: ChangeControlRules,
    pub main_control_group: Option<(BTreeSet<Identifier>, RequiredSigners)>,
    pub main_control_group_can_be_modified: ChangeControlRules,
    pub balance_can_be_increased: ChangeControlRules,
    pub balance_can_be_destroyed: ChangeControlRules,
}