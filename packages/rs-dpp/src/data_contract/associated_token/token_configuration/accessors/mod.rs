pub mod v0;

use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::GroupContractPosition;
use std::collections::BTreeSet;

/// Implementing TokenConfigurationV0Getters for TokenConfiguration
impl TokenConfigurationV0Getters for TokenConfiguration {
    /// Returns a reference to the conventions.
    fn conventions(&self) -> &TokenConfigurationConvention {
        match self {
            TokenConfiguration::V0(v0) => v0.conventions(),
        }
    }

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConvention {
        match self {
            TokenConfiguration::V0(v0) => v0.conventions_mut(),
        }
    }

    fn conventions_change_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.conventions_change_rules(),
        }
    }

    /// Returns the base supply.
    fn base_supply(&self) -> TokenAmount {
        match self {
            TokenConfiguration::V0(v0) => v0.base_supply(),
        }
    }

    /// Returns if we keep history.
    fn keeps_history(&self) -> &TokenKeepsHistoryRules {
        match self {
            TokenConfiguration::V0(v0) => v0.keeps_history(),
        }
    }

    /// Returns if we keep history.
    fn keeps_history_mut(&mut self) -> &mut TokenKeepsHistoryRules {
        match self {
            TokenConfiguration::V0(v0) => v0.keeps_history_mut(),
        }
    }

    /// Returns if we start as paused.
    fn start_as_paused(&self) -> bool {
        match self {
            TokenConfiguration::V0(v0) => v0.start_as_paused(),
        }
    }

    fn is_allowed_transfer_to_frozen_balance(&self) -> bool {
        match self {
            TokenConfiguration::V0(v0) => v0.is_allowed_transfer_to_frozen_balance(),
        }
    }

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<TokenAmount> {
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

    fn distribution_rules(&self) -> &TokenDistributionRules {
        match self {
            TokenConfiguration::V0(v0) => v0.distribution_rules(),
        }
    }

    fn distribution_rules_mut(&mut self) -> &mut TokenDistributionRules {
        match self {
            TokenConfiguration::V0(v0) => v0.distribution_rules_mut(),
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

    fn destroy_frozen_funds_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.destroy_frozen_funds_rules(),
        }
    }

    fn emergency_action_rules(&self) -> &ChangeControlRules {
        match self {
            TokenConfiguration::V0(v0) => v0.emergency_action_rules(),
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

    /// Returns all group positions used in the token configuration
    fn all_used_group_positions(&self) -> (BTreeSet<GroupContractPosition>, bool) {
        match self {
            TokenConfiguration::V0(v0) => v0.all_used_group_positions(),
        }
    }

    /// Returns all the change contract rules, including those from the distribution rules
    fn all_change_control_rules(&self) -> Vec<(&str, &ChangeControlRules)> {
        match self {
            TokenConfiguration::V0(v0) => v0.all_change_control_rules(),
        }
    }

    /// Returns the token description.
    fn description(&self) -> &Option<String> {
        match self {
            TokenConfiguration::V0(v0) => v0.description(),
        }
    }
}

/// Implementing TokenConfigurationV0Setters for TokenConfiguration
impl TokenConfigurationV0Setters for TokenConfiguration {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConvention) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_conventions(conventions),
        }
    }

    /// Sets the conventions change rules.
    fn set_conventions_change_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_conventions_change_rules(rules),
        }
    }

    /// Allow or not a transfer and mint tokens to frozen identity token balances
    fn allow_transfer_to_frozen_balance(&mut self, allow: bool) {
        match self {
            TokenConfiguration::V0(v0) => v0.allow_transfer_to_frozen_balance(allow),
        }
    }

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: u64) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_base_supply(base_supply),
        }
    }

    /// Sets if we should start as paused. Meaning transfers will not work till unpaused
    fn set_start_as_paused(&mut self, start_as_paused: bool) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_start_as_paused(start_as_paused),
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

    fn set_distribution_rules(&mut self, rules: TokenDistributionRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_distribution_rules(rules),
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

    fn set_destroy_frozen_funds_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_destroy_frozen_funds_rules(rules),
        }
    }

    fn set_emergency_action_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_emergency_action_rules(rules),
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

    /// Sets the token description.
    fn set_description(&mut self, description: Option<String>) {
        match self {
            TokenConfiguration::V0(v0) => v0.set_description(description),
        }
    }
}
