use serde::{Deserialize, Serialize};
use platform_value::Identifier;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};

pub type RequiredSigners = u8;
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum AuthorizedActionTakers {
    None,
    ContractOwner,
    MainGroup,
    SpecifiedIdentities(Vec<Identifier>, RequiredSigners)
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV0 {
    pub max_supply: u64,
    pub max_supply_can_be_increased: AuthorizedActionTakers,
    pub main_control_group: Option<(Vec<Identifier>, RequiredSigners)>,
    pub main_control_group_can_be_modified: AuthorizedActionTakers,
    pub balance_can_be_increased: AuthorizedActionTakers,
    pub balance_can_be_destroyed: AuthorizedActionTakers,
    pub authorized_action_takers_level_can_be_changed: AuthorizedActionTakers,
    pub authorized_action_takers_
}