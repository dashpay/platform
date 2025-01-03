use crate::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use crate::data_contract::associated_token::token_configuration::v0::{
    TokenConfigurationConventionV0, TokenConfigurationV0,
};
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::group::RequiredSigners;
use platform_value::Identifier;
use std::collections::BTreeSet;

/// Implementing `TokenConfigurationV0Getters` for `TokenConfigurationV0`
impl TokenConfigurationV0Getters for TokenConfigurationV0 {
    /// Returns a reference to the conventions.
    fn conventions(&self) -> &TokenConfigurationConventionV0 {
        &self.conventions
    }

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConventionV0 {
        &mut self.conventions
    }

    /// Returns the base supply.
    fn base_supply(&self) -> u64 {
        self.base_supply
    }

    /// Returns if we keep history.
    fn keeps_history(&self) -> bool {
        self.keeps_history
    }

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<u64> {
        self.max_supply
    }

    /// Returns the max supply change rules.
    fn max_supply_change_rules(&self) -> &ChangeControlRules {
        &self.max_supply_change_rules
    }

    /// Returns the new tokens destination identity.
    fn new_tokens_destination_identity(&self) -> Option<Identifier> {
        self.new_tokens_destination_identity
    }

    /// Returns the new tokens destination identity rules.
    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules {
        &self.new_tokens_destination_identity_rules
    }

    /// Returns whether minting allows choosing a destination.
    fn minting_allow_choosing_destination(&self) -> bool {
        self.minting_allow_choosing_destination
    }

    /// Returns the rules for minting destination selection.
    fn minting_allow_choosing_destination_rules(&self) -> &ChangeControlRules {
        &self.minting_allow_choosing_destination_rules
    }

    /// Returns the manual minting rules.
    fn manual_minting_rules(&self) -> &ChangeControlRules {
        &self.manual_minting_rules
    }

    /// Returns the manual burning rules.
    fn manual_burning_rules(&self) -> &ChangeControlRules {
        &self.manual_burning_rules
    }

    /// Returns the freeze rules.
    fn freeze_rules(&self) -> &ChangeControlRules {
        &self.freeze_rules
    }

    /// Returns the unfreeze rules.
    fn unfreeze_rules(&self) -> &ChangeControlRules {
        &self.unfreeze_rules
    }

    /// Returns the main control group.
    fn main_control_group(&self) -> Option<&(BTreeSet<Identifier>, RequiredSigners)> {
        self.main_control_group.as_ref()
    }

    /// Returns the main control group can be modified.
    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers {
        &self.main_control_group_can_be_modified
    }
}

/// Implementing `TokenConfigurationV0Setters` for `TokenConfigurationV0`
impl TokenConfigurationV0Setters for TokenConfigurationV0 {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConventionV0) {
        self.conventions = conventions;
    }

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: u64) {
        self.base_supply = base_supply;
    }

    /// Sets the maximum supply.
    fn set_max_supply(&mut self, max_supply: Option<u64>) {
        self.max_supply = max_supply;
    }

    /// Sets the max supply change rules.
    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules) {
        self.max_supply_change_rules = rules;
    }

    /// Sets the new tokens destination identity.
    fn set_new_tokens_destination_identity(&mut self, id: Option<Identifier>) {
        self.new_tokens_destination_identity = id;
    }

    /// Sets the new tokens destination identity rules.
    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules) {
        self.new_tokens_destination_identity_rules = rules;
    }

    /// Sets the manual minting rules.
    fn set_manual_minting_rules(&mut self, rules: ChangeControlRules) {
        self.manual_minting_rules = rules;
    }

    /// Sets the manual burning rules.
    fn set_manual_burning_rules(&mut self, rules: ChangeControlRules) {
        self.manual_burning_rules = rules;
    }

    /// Sets the freeze rules.
    fn set_freeze_rules(&mut self, rules: ChangeControlRules) {
        self.freeze_rules = rules;
    }

    /// Sets the unfreeze rules.
    fn set_unfreeze_rules(&mut self, rules: ChangeControlRules) {
        self.unfreeze_rules = rules;
    }

    /// Sets the main control group.
    fn set_main_control_group(&mut self, group: Option<(BTreeSet<Identifier>, RequiredSigners)>) {
        self.main_control_group = group;
    }

    /// Sets the main control group can be modified.
    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers) {
        self.main_control_group_can_be_modified = action_takers;
    }

    /// Sets whether minting allows choosing a destination.
    fn set_minting_allow_choosing_destination(&mut self, value: bool) {
        self.minting_allow_choosing_destination = value;
    }

    /// Sets the rules for minting destination selection.
    fn set_minting_allow_choosing_destination_rules(&mut self, rules: ChangeControlRules) {
        self.minting_allow_choosing_destination_rules = rules;
    }
}
