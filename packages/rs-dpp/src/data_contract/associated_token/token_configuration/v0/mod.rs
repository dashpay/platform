use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::multi_identity_events::ActionTaker;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use serde_json::map::BTreeMap;
use std::collections::{BTreeMap, BTreeSet};

pub type RequiredSigners = u8;
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub enum AuthorizedActionTakers {
    NoOne,
    ContractOwner,
    MainGroup,
    SpecifiedIdentities(BTreeSet<Identifier>, RequiredSigners),
}

impl AuthorizedActionTakers {
    pub fn allowed_for_action_taker(
        &self,
        contract_owner_id: &Identifier,
        main_group: &(BTreeSet<Identifier>, RequiredSigners),
        action_taker: &ActionTaker,
    ) -> bool {
        match self {
            AuthorizedActionTakers::NoOne => false,
            AuthorizedActionTakers::ContractOwner => match action_taker {
                ActionTaker::SingleIdentity(action_taker) => action_taker == contract_owner_id,
                ActionTaker::SpecifiedIdentities(action_takers) => {
                    action_takers.contains(contract_owner_id)
                }
            },
            AuthorizedActionTakers::MainGroup => match action_taker {
                ActionTaker::SingleIdentity(_) => false,
                ActionTaker::SpecifiedIdentities(action_takers) => {
                    let authorized_action_takers_count =
                        main_group.0.intersection(action_takers).count();
                    if authorized_action_takers_count > 255 {
                        return false;
                    }
                    authorized_action_takers_count as u8 >= main_group.1
                }
            },
            AuthorizedActionTakers::SpecifiedIdentities(
                specified_authorized_identities,
                required_signers_count,
            ) => match action_taker {
                ActionTaker::SingleIdentity(_) => false,
                ActionTaker::SpecifiedIdentities(action_takers) => {
                    let authorized_action_takers_count = specified_authorized_identities
                        .intersection(action_takers)
                        .count();
                    if authorized_action_takers_count > 255 {
                        return false;
                    }
                    authorized_action_takers_count as u8 >= *required_signers_count
                }
            },
        }
    }
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
pub struct ChangeControlRules {
    /// This is who is authorized to make such a change
    authorized_to_make_change: AuthorizedActionTakers,
    /// This is who is authorized to make such a change to the people authorized to make a change
    authorized_to_change_authorized_action_takers: AuthorizedActionTakers,
    /// Are we allowed to change to None in the future
    changing_authorized_action_takers_to_no_one_allowed: bool,
    /// Are we allowed to change to None in the future
    changing_authorized_action_takers_to_contract_owner_allowed: bool,
}

impl ChangeControlRules {
    pub fn can_change_to(
        &self,
        other: &ChangeControlRules,
        contract_owner_id: &Identifier,
        main_group: &(BTreeSet<Identifier>, RequiredSigners),
        action_taker: &ActionTaker,
    ) -> bool {
        // First, check if the action taker is allowed to make any changes at all
        if !self.authorized_to_make_change.allowed_for_action_taker(
            contract_owner_id,
            main_group,
            action_taker,
        ) {
            return false;
        }

        // Check if authorized_to_make_change is being modified
        if self.authorized_to_make_change != other.authorized_to_make_change {
            // Changing the authorized action takers requires the action_taker to be allowed by
            // authorized_to_change_authorized_action_takers in the current rules
            if !self
                .authorized_to_change_authorized_action_takers
                .allowed_for_action_taker(contract_owner_id, main_group, action_taker)
            {
                return false;
            }

            // If we are changing to NoOne, ensure it's allowed
            if let AuthorizedActionTakers::NoOne = other.authorized_to_make_change {
                if !self.changing_authorized_action_takers_to_no_one_allowed {
                    return false;
                }
            }

            // If we are changing to ContractOwner, ensure it's allowed
            if let AuthorizedActionTakers::ContractOwner = other.authorized_to_make_change {
                if !self.changing_authorized_action_takers_to_contract_owner_allowed {
                    return false;
                }
            }
        }

        // Check if authorized_to_change_authorized_action_takers is being modified
        if self.authorized_to_change_authorized_action_takers
            != other.authorized_to_change_authorized_action_takers
        {
            // Must be allowed by the current authorized_to_change_authorized_action_takers
            if !self
                .authorized_to_change_authorized_action_takers
                .allowed_for_action_taker(contract_owner_id, main_group, action_taker)
            {
                return false;
            }

            // If we are changing to NoOne, ensure it's allowed
            if let AuthorizedActionTakers::NoOne =
                other.authorized_to_change_authorized_action_takers
            {
                if !self.changing_authorized_action_takers_to_no_one_allowed {
                    return false;
                }
            }

            // If we are changing to ContractOwner, ensure it's allowed
            if let AuthorizedActionTakers::ContractOwner =
                other.authorized_to_change_authorized_action_takers
            {
                if !self.changing_authorized_action_takers_to_contract_owner_allowed {
                    return false;
                }
            }
        }

        // If we reach here, the changes are allowed
        true
    }
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationLocalizationsV0 {
    pub singular_form: String,
    pub plural_form: String,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationConventionV0 {
    pub should_capitalize: bool,
    pub localizations: BTreeMap<String, TokenConfigurationLocalizationsV0>,
    pub decimals: u16,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV0 {
    pub conventions: TokenConfigurationConventionV0,
    /// The supply at the creation of the token
    pub base_supply: u64,
    /// The maximum supply the token can ever have
    pub max_supply: Option<u64>,
    /// Who can change the max supply
    /// Even if set no one can ever change this under the base supply
    pub max_supply_change_rules: ChangeControlRules,
    pub new_tokens_destination_identity: Option<Identifier>,
    pub new_tokens_destination_identity_rules: ChangeControlRules,
    pub manual_minting_rules: ChangeControlRules,
    pub manual_burning_rules: ChangeControlRules,
    pub main_control_group: Option<(BTreeSet<Identifier>, RequiredSigners)>,
    pub main_control_group_can_be_modified: AuthorizedActionTakers,
}
