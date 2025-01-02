mod accessors;

use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::group::RequiredSigners;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationLocalizationsV0 {
    pub should_capitalize: bool,
    pub singular_form: String,
    pub plural_form: String,
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationConventionV0 {
    #[serde(default)]
    pub localizations: BTreeMap<String, TokenConfigurationLocalizationsV0>,
    #[serde(default = "default_decimals")]
    pub decimals: u16,
}

// Default function for `decimals`
fn default_decimals() -> u16 {
    8 // Default value for decimals
}

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV0 {
    pub conventions: TokenConfigurationConventionV0,
    /// The supply at the creation of the token
    pub base_supply: u64,
    /// The maximum supply the token can ever have
    #[serde(default)]
    pub max_supply: Option<u64>,
    /// Do we keep history, default is true.
    #[serde(default = "default_keeps_history")]
    pub keeps_history: bool,
    /// Who can change the max supply
    /// Even if set no one can ever change this under the base supply
    #[serde(default = "default_change_control_rules")]
    pub max_supply_change_rules: ChangeControlRules,
    #[serde(default)]
    pub new_tokens_destination_identity: Option<Identifier>,
    #[serde(default = "default_change_control_rules")]
    pub new_tokens_destination_identity_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub manual_minting_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub manual_burning_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub freeze_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub unfreeze_rules: ChangeControlRules,
    #[serde(default)]
    pub main_control_group: Option<(BTreeSet<Identifier>, RequiredSigners)>,
    #[serde(default)]
    pub main_control_group_can_be_modified: AuthorizedActionTakers,
}

// Default function for `keeps_history`
fn default_keeps_history() -> bool {
    true // Default to `true` for keeps_history
}

fn default_change_control_rules() -> ChangeControlRules {
    ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::NoOne,
        authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_authorized_action_takers_to_contract_owner_allowed: false,
    })
}

impl fmt::Display for TokenConfigurationV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Using debug formatting for nested fields
        write!(
            f,
            "TokenConfigurationV0 {{ conventions: {:?}, base_supply: {}, max_supply: {:?}, max_supply_change_rules: {:?}, new_tokens_destination_identity: {:?}, new_tokens_destination_identity_rules: {:?}, manual_minting_rules: {:?}, manual_burning_rules: {:?}, freeze_rules: {:?}, unfreeze_rules: {:?}, main_control_group: {:?}, main_control_group_can_be_modified: {:?} }}",
            self.conventions,
            self.base_supply,
            self.max_supply,
            self.max_supply_change_rules,
            self.new_tokens_destination_identity,
            self.new_tokens_destination_identity_rules,
            self.manual_minting_rules,
            self.manual_burning_rules,
            self.freeze_rules,
            self.unfreeze_rules,
            self.main_control_group,
            self.main_control_group_can_be_modified
        )
    }
}

impl TokenConfigurationV0 {
    pub fn default_most_restrictive() -> Self {
        Self {
            conventions: TokenConfigurationConventionV0 {
                localizations: Default::default(),
                decimals: 8,
            },
            base_supply: 100000,
            max_supply: None,
            keeps_history: true,
            max_supply_change_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }
            .into(),
            new_tokens_destination_identity: None,
            new_tokens_destination_identity_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }
            .into(),
            manual_minting_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }
            .into(),
            manual_burning_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }
            .into(),
            freeze_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }
            .into(),
            unfreeze_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                authorized_to_change_authorized_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_authorized_action_takers_to_contract_owner_allowed: false,
            }
            .into(),
            main_control_group: None,
            main_control_group_can_be_modified: AuthorizedActionTakers::NoOne,
        }
    }
}
