use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use crate::data_contract::associated_token::token_configuration::v0::{
    TokenConfigurationConvention, TokenConfigurationV0,
};
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::GroupContractPosition;
use std::collections::BTreeSet;

/// Implementing `TokenConfigurationV0Getters` for `TokenConfigurationV0`
impl TokenConfigurationV0Getters for TokenConfigurationV0 {
    /// Returns a reference to the conventions.
    fn conventions(&self) -> &TokenConfigurationConvention {
        &self.conventions
    }

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConvention {
        &mut self.conventions
    }

    /// Returns the conventions change rules.
    fn conventions_change_rules(&self) -> &ChangeControlRules {
        &self.conventions_change_rules
    }

    /// Returns the base supply.
    fn base_supply(&self) -> TokenAmount {
        self.base_supply
    }

    /// Returns if we keep history.
    fn keeps_history(&self) -> bool {
        self.keeps_history
    }

    /// Returns if we start off as paused
    fn start_as_paused(&self) -> bool {
        self.start_as_paused
    }

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<TokenAmount> {
        self.max_supply
    }

    /// Returns the max supply change rules.
    fn max_supply_change_rules(&self) -> &ChangeControlRules {
        &self.max_supply_change_rules
    }

    fn distribution_rules(&self) -> &TokenDistributionRules {
        &self.distribution_rules
    }

    fn distribution_rules_mut(&mut self) -> &mut TokenDistributionRules {
        &mut self.distribution_rules
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

    /// Returns the `destroy frozen funds` rules.
    fn destroy_frozen_funds_rules(&self) -> &ChangeControlRules {
        &self.destroy_frozen_funds_rules
    }

    /// Returns the emergency action rules.
    fn emergency_action_rules(&self) -> &ChangeControlRules {
        &self.emergency_action_rules
    }

    /// Returns the main control group.
    fn main_control_group(&self) -> Option<GroupContractPosition> {
        self.main_control_group
    }

    /// Returns the main control group can be modified.
    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers {
        &self.main_control_group_can_be_modified
    }

    /// Returns all group positions used in the token configuration
    fn all_used_group_positions(&self) -> BTreeSet<GroupContractPosition> {
        let mut group_positions = BTreeSet::new();

        // Add the main control group if it exists
        if let Some(main_group_position) = self.main_control_group {
            group_positions.insert(main_group_position);
        }

        // Helper function to extract group positions from `AuthorizedActionTakers`
        let mut add_from_authorized_action_takers = |authorized_takers: &AuthorizedActionTakers| {
            if let AuthorizedActionTakers::Group(group_position) = authorized_takers {
                group_positions.insert(*group_position);
            }
        };

        // Add positions from change control rules
        let mut add_from_change_control_rules = |rules: &ChangeControlRules| {
            add_from_authorized_action_takers(rules.authorized_to_make_change_action_takers());
            add_from_authorized_action_takers(rules.admin_action_takers());
        };

        // Apply the helper to all fields containing `ChangeControlRules`
        add_from_change_control_rules(&self.max_supply_change_rules);
        add_from_change_control_rules(&self.conventions_change_rules);
        add_from_change_control_rules(
            self.distribution_rules
                .new_tokens_destination_identity_rules(),
        );
        add_from_change_control_rules(
            self.distribution_rules
                .minting_allow_choosing_destination_rules(),
        );
        add_from_change_control_rules(self.distribution_rules.perpetual_distribution_rules());
        add_from_change_control_rules(&self.manual_minting_rules);
        add_from_change_control_rules(&self.manual_burning_rules);
        add_from_change_control_rules(&self.freeze_rules);
        add_from_change_control_rules(&self.unfreeze_rules);
        add_from_change_control_rules(&self.destroy_frozen_funds_rules);
        add_from_change_control_rules(&self.emergency_action_rules);

        // Add positions from the `main_control_group_can_be_modified` field
        add_from_authorized_action_takers(&self.main_control_group_can_be_modified);

        group_positions
    }
}

/// Implementing `TokenConfigurationV0Setters` for `TokenConfigurationV0`
impl TokenConfigurationV0Setters for TokenConfigurationV0 {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConvention) {
        self.conventions = conventions;
    }

    /// Sets the new conventions change rules.
    fn set_conventions_change_rules(&mut self, rules: ChangeControlRules) {
        self.conventions_change_rules = rules;
    }

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: TokenAmount) {
        self.base_supply = base_supply;
    }

    /// Sets the maximum supply.
    fn set_max_supply(&mut self, max_supply: Option<TokenAmount>) {
        self.max_supply = max_supply;
    }

    /// Sets the max supply change rules.
    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules) {
        self.max_supply_change_rules = rules;
    }

    fn set_distribution_rules(&mut self, rules: TokenDistributionRules) {
        self.distribution_rules = rules;
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

    /// Sets the destroy frozen funds rules.
    fn set_destroy_frozen_funds_rules(&mut self, rules: ChangeControlRules) {
        self.destroy_frozen_funds_rules = rules;
    }

    /// Sets the emergency action rules.
    fn set_emergency_action_rules(&mut self, rules: ChangeControlRules) {
        self.emergency_action_rules = rules;
    }

    /// Sets the main control group.
    fn set_main_control_group(&mut self, group: Option<GroupContractPosition>) {
        self.main_control_group = group;
    }

    /// Sets the main control group can be modified.
    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers) {
        self.main_control_group_can_be_modified = action_takers;
    }
}
