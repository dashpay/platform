mod accessors;

use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::group::RequiredSigners;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
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

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationConventionV0 {
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
    pub freeze_rules: ChangeControlRules,
    pub unfreeze_rules: ChangeControlRules,
    pub main_control_group: Option<(BTreeSet<Identifier>, RequiredSigners)>,
    pub main_control_group_can_be_modified: AuthorizedActionTakers,
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
