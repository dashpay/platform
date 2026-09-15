use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use crate::data_contract::associated_token::token_configuration::accessors::v1::{
    TokenConfigurationV1Getters, TokenConfigurationV1Setters,
};
use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::GroupContractPosition;
use std::collections::BTreeSet;

/// Every V0 getter reads through to the nested `base` configuration.
impl TokenConfigurationV0Getters for TokenConfigurationV1 {
    fn conventions(&self) -> &TokenConfigurationConvention {
        self.base.conventions()
    }

    fn conventions_mut(&mut self) -> &mut TokenConfigurationConvention {
        self.base.conventions_mut()
    }

    fn conventions_change_rules(&self) -> &ChangeControlRules {
        self.base.conventions_change_rules()
    }

    fn base_supply(&self) -> TokenAmount {
        self.base.base_supply()
    }

    fn keeps_history(&self) -> &TokenKeepsHistoryRules {
        self.base.keeps_history()
    }

    fn keeps_history_mut(&mut self) -> &mut TokenKeepsHistoryRules {
        self.base.keeps_history_mut()
    }

    fn start_as_paused(&self) -> bool {
        self.base.start_as_paused()
    }

    fn is_allowed_transfer_to_frozen_balance(&self) -> bool {
        self.base.is_allowed_transfer_to_frozen_balance()
    }

    fn max_supply(&self) -> Option<TokenAmount> {
        self.base.max_supply()
    }

    fn max_supply_change_rules(&self) -> &ChangeControlRules {
        self.base.max_supply_change_rules()
    }

    fn distribution_rules(&self) -> &TokenDistributionRules {
        self.base.distribution_rules()
    }

    fn distribution_rules_mut(&mut self) -> &mut TokenDistributionRules {
        self.base.distribution_rules_mut()
    }

    fn manual_minting_rules(&self) -> &ChangeControlRules {
        self.base.manual_minting_rules()
    }

    fn manual_burning_rules(&self) -> &ChangeControlRules {
        self.base.manual_burning_rules()
    }

    fn freeze_rules(&self) -> &ChangeControlRules {
        self.base.freeze_rules()
    }

    fn unfreeze_rules(&self) -> &ChangeControlRules {
        self.base.unfreeze_rules()
    }

    fn destroy_frozen_funds_rules(&self) -> &ChangeControlRules {
        self.base.destroy_frozen_funds_rules()
    }

    fn emergency_action_rules(&self) -> &ChangeControlRules {
        self.base.emergency_action_rules()
    }

    fn main_control_group(&self) -> Option<GroupContractPosition> {
        self.base.main_control_group()
    }

    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers {
        self.base.main_control_group_can_be_modified()
    }

    fn all_used_group_positions(&self) -> (BTreeSet<GroupContractPosition>, bool) {
        self.base.all_used_group_positions()
    }

    fn all_change_control_rules(&self) -> Vec<(&str, &ChangeControlRules)> {
        self.base.all_change_control_rules()
    }

    fn description(&self) -> &Option<String> {
        self.base.description()
    }
}

/// Every V0 setter writes through to the nested `base` configuration.
impl TokenConfigurationV0Setters for TokenConfigurationV1 {
    fn set_conventions(&mut self, conventions: TokenConfigurationConvention) {
        self.base.set_conventions(conventions)
    }

    fn set_conventions_change_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_conventions_change_rules(rules)
    }

    fn allow_transfer_to_frozen_balance(&mut self, allow: bool) {
        self.base.allow_transfer_to_frozen_balance(allow)
    }

    fn set_base_supply(&mut self, base_supply: TokenAmount) {
        self.base.set_base_supply(base_supply)
    }

    fn set_start_as_paused(&mut self, start_as_paused: bool) {
        self.base.set_start_as_paused(start_as_paused)
    }

    fn set_max_supply(&mut self, max_supply: Option<TokenAmount>) {
        self.base.set_max_supply(max_supply)
    }

    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_max_supply_change_rules(rules)
    }

    fn set_distribution_rules(&mut self, rules: TokenDistributionRules) {
        self.base.set_distribution_rules(rules)
    }

    fn set_manual_minting_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_manual_minting_rules(rules)
    }

    fn set_manual_burning_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_manual_burning_rules(rules)
    }

    fn set_freeze_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_freeze_rules(rules)
    }

    fn set_unfreeze_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_unfreeze_rules(rules)
    }

    fn set_destroy_frozen_funds_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_destroy_frozen_funds_rules(rules)
    }

    fn set_emergency_action_rules(&mut self, rules: ChangeControlRules) {
        self.base.set_emergency_action_rules(rules)
    }

    fn set_main_control_group(&mut self, group: Option<GroupContractPosition>) {
        self.base.set_main_control_group(group)
    }

    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers) {
        self.base
            .set_main_control_group_can_be_modified(action_takers)
    }

    fn set_description(&mut self, description: Option<String>) {
        self.base.set_description(description)
    }
}

impl TokenConfigurationV1Getters for TokenConfigurationV1 {
    fn has_shielded_pool(&self) -> bool {
        self.has_shielded_pool
    }
}

impl TokenConfigurationV1Setters for TokenConfigurationV1 {
    fn set_has_shielded_pool(&mut self, has_shielded_pool: bool) {
        self.has_shielded_pool = has_shielded_pool;
    }
}
