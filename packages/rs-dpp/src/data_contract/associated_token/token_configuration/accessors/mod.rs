pub mod v0;

use crate::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationConventionV0;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::group::RequiredSigners;
use crate::data_contract::GroupContractPosition;
use platform_value::Identifier;
use std::collections::BTreeSet;

/// Implementing TokenConfigurationV0Getters for TokenConfiguration
impl TokenConfigurationV0Getters for TokenConfiguration {
    /// Returns a reference to the conventions.
    fn conventions(&self) -> &TokenConfigurationConventionV0 {
        match self {
            TokenConfiguration::V0(v0) => v0.conventions(),
        }
    }

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConventionV0 {
        match self {
            TokenConfiguration::V0(v0) => v0.conventions_mut(),
        }
    }

    /// Returns the base supply.
    fn base_supply(&self) -> u64 {
        match self {
            TokenConfiguration::V0(v0) => v0.base_supply(),
        }
    }

    /// Returns if we keep history.
    fn keeps_history(&self) -> bool {
        match self {
            TokenConfiguration::V0(v0) => v0.keeps_history(),
        }
    }

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<u64> {
        match self {
            TokenConfiguration::V0(v0) => v0.max_supply(),
        }
    }

    /// Returns the max supply change rules.
    fn max_supply_change_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.max_supply_change_rules(),
        }
    }

    /// Returns the new tokens destination identity.
    fn new_tokens_destination_identity(&self) -> Option<Identifier> {
        match self {
            TokenConfiguration::V0(v0) => v0.new_tokens_destination_identity(),
        }
    }

    /// Returns the new tokens destination identity rules.
    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.new_tokens_destination_identity_rules(),
        }
    }
    /// Returns whether minting allows choosing a destination.
    fn minting_allow_choosing_destination(&self) -> bool {
        match self {
            TokenConfiguration::V0(v0) => v0.minting_allow_choosing_destination(),
        }
    }

    /// Returns the rules for minting destination selection.
    fn minting_allow_choosing_destination_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.minting_allow_choosing_destination_rules(),
        }
    }

    /// Returns the manual minting rules.
    fn manual_minting_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.manual_minting_rules(),
        }
    }

    /// Returns the manual burning rules.
    fn manual_burning_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.manual_burning_rules(),
        }
    }

    /// Returns the freeze rules.
    fn freeze_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.freeze_rules(),
        }
    }

    /// Returns the unfreeze rules.
    fn unfreeze_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.unfreeze_rules(),
        }
    }

    /// Returns the main control group.
    fn main_control_group(&self) -> Option<GroupContractPosition> {
        match self {
            TokenConfiguration::V0(v0) => v0.main_control_group(),
        }
    }

    /// Returns the main control group can be modified.
    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers {
        match self {
            TokenConfiguration::V0(v0) => v0.main_control_group_can_be_modified(),
        }
    }
}

/// Implementing TokenConfigurationV0Setters for TokenConfiguration
impl TokenConfigurationV0Setters for TokenConfiguration {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConventionV0) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_conventions(conventions),
        }
    }

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: u64) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_base_supply(base_supply),
        }
    }

    /// Sets the maximum supply.
    fn set_max_supply(&mut self, max_supply: Option<u64>) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_max_supply(max_supply),
        }
    }

    /// Sets the max supply change rules.
    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_max_supply_change_rules(rules),
        }
    }

    /// Sets the new tokens destination identity.
    fn set_new_tokens_destination_identity(&mut self, id: Option<Identifier>) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_new_tokens_destination_identity(id),
        }
    }

    /// Sets the new tokens destination identity rules.
    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_new_tokens_destination_identity_rules(rules),
        }
    }

    /// Sets the manual minting rules.
    fn set_manual_minting_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_manual_minting_rules(rules),
        }
    }

    /// Sets the manual burning rules.
    fn set_manual_burning_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_manual_burning_rules(rules),
        }
    }

    /// Sets the freeze rules.
    fn set_freeze_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_freeze_rules(rules),
        }
    }

    /// Sets the unfreeze rules.
    fn set_unfreeze_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_unfreeze_rules(rules),
        }
    }

    /// Sets the main control group.
    fn set_main_control_group(&mut self, group: Option<GroupContractPosition>) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_main_control_group(group),
        }
    }

    /// Sets the main control group can be modified.
    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_main_control_group_can_be_modified(action_takers),
        }
    }

    fn set_minting_allow_choosing_destination(&mut self, value: bool) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_minting_allow_choosing_destination(value),
        }
    }

    fn set_minting_allow_choosing_destination_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_minting_allow_choosing_destination_rules(rules),
        }
    }
}
