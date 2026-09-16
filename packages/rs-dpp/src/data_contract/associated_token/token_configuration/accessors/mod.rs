pub mod v0;
pub mod v1;

use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration::accessors::v0::{
    TokenConfigurationV0Getters, TokenConfigurationV0Setters,
};
use crate::data_contract::associated_token::token_configuration::accessors::v1::{
    TokenConfigurationV1Getters, TokenConfigurationV1Setters,
};
use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
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
        self.as_v0().conventions()
    }

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConvention {
        self.as_v0_mut().conventions_mut()
    }

    fn conventions_change_rules(&self) -> &ChangeControlRules {
        self.as_v0().conventions_change_rules()
    }

    /// Returns the base supply.
    fn base_supply(&self) -> TokenAmount {
        self.as_v0().base_supply()
    }

    /// Returns if we keep history.
    fn keeps_history(&self) -> &TokenKeepsHistoryRules {
        self.as_v0().keeps_history()
    }

    /// Returns if we keep history.
    fn keeps_history_mut(&mut self) -> &mut TokenKeepsHistoryRules {
        self.as_v0_mut().keeps_history_mut()
    }

    /// Returns if we start as paused.
    fn start_as_paused(&self) -> bool {
        self.as_v0().start_as_paused()
    }

    fn is_allowed_transfer_to_frozen_balance(&self) -> bool {
        self.as_v0().is_allowed_transfer_to_frozen_balance()
    }

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<TokenAmount> {
        self.as_v0().max_supply()
    }

    /// Returns the max supply change rules.
    fn max_supply_change_rules(&self) -> &ChangeControlRules {
        self.as_v0().max_supply_change_rules()
    }

    fn distribution_rules(&self) -> &TokenDistributionRules {
        self.as_v0().distribution_rules()
    }

    fn distribution_rules_mut(&mut self) -> &mut TokenDistributionRules {
        self.as_v0_mut().distribution_rules_mut()
    }

    /// Returns the manual minting rules.
    fn manual_minting_rules(&self) -> &ChangeControlRules {
        self.as_v0().manual_minting_rules()
    }

    /// Returns the manual burning rules.
    fn manual_burning_rules(&self) -> &ChangeControlRules {
        self.as_v0().manual_burning_rules()
    }

    /// Returns the freeze rules.
    fn freeze_rules(&self) -> &ChangeControlRules {
        self.as_v0().freeze_rules()
    }

    /// Returns the unfreeze rules.
    fn unfreeze_rules(&self) -> &ChangeControlRules {
        self.as_v0().unfreeze_rules()
    }

    fn destroy_frozen_funds_rules(&self) -> &ChangeControlRules {
        self.as_v0().destroy_frozen_funds_rules()
    }

    fn emergency_action_rules(&self) -> &ChangeControlRules {
        self.as_v0().emergency_action_rules()
    }

    /// Returns the main control group.
    fn main_control_group(&self) -> Option<GroupContractPosition> {
        self.as_v0().main_control_group()
    }

    /// Returns the main control group can be modified.
    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers {
        self.as_v0().main_control_group_can_be_modified()
    }

    /// Returns all group positions used in the token configuration
    fn all_used_group_positions(&self) -> (BTreeSet<GroupContractPosition>, bool) {
        self.as_v0().all_used_group_positions()
    }

    /// Returns all the change contract rules, including those from the distribution rules
    fn all_change_control_rules(&self) -> Vec<(&str, &ChangeControlRules)> {
        self.as_v0().all_change_control_rules()
    }

    /// Returns the token description.
    fn description(&self) -> &Option<String> {
        self.as_v0().description()
    }
}

/// Implementing TokenConfigurationV0Setters for TokenConfiguration
impl TokenConfigurationV0Setters for TokenConfiguration {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConvention) {
        self.as_v0_mut().set_conventions(conventions)
    }

    /// Sets the conventions change rules.
    fn set_conventions_change_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_conventions_change_rules(rules)
    }

    /// Allow or not a transfer and mint tokens to frozen identity token balances
    fn allow_transfer_to_frozen_balance(&mut self, allow: bool) {
        self.as_v0_mut().allow_transfer_to_frozen_balance(allow)
    }

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: u64) {
        self.as_v0_mut().set_base_supply(base_supply)
    }

    /// Sets if we should start as paused. Meaning transfers will not work till unpaused
    fn set_start_as_paused(&mut self, start_as_paused: bool) {
        self.as_v0_mut().set_start_as_paused(start_as_paused)
    }

    /// Sets the maximum supply.
    fn set_max_supply(&mut self, max_supply: Option<u64>) {
        self.as_v0_mut().set_max_supply(max_supply)
    }

    /// Sets the max supply change rules.
    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_max_supply_change_rules(rules)
    }

    fn set_distribution_rules(&mut self, rules: TokenDistributionRules) {
        self.as_v0_mut().set_distribution_rules(rules)
    }

    /// Sets the manual minting rules.
    fn set_manual_minting_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_manual_minting_rules(rules)
    }

    /// Sets the manual burning rules.
    fn set_manual_burning_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_manual_burning_rules(rules)
    }

    /// Sets the freeze rules.
    fn set_freeze_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_freeze_rules(rules)
    }

    /// Sets the unfreeze rules.
    fn set_unfreeze_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_unfreeze_rules(rules)
    }

    fn set_destroy_frozen_funds_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_destroy_frozen_funds_rules(rules)
    }

    fn set_emergency_action_rules(&mut self, rules: ChangeControlRules) {
        self.as_v0_mut().set_emergency_action_rules(rules)
    }

    /// Sets the main control group.
    fn set_main_control_group(&mut self, group: Option<GroupContractPosition>) {
        self.as_v0_mut().set_main_control_group(group)
    }

    /// Sets the main control group can be modified.
    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers) {
        self.as_v0_mut()
            .set_main_control_group_can_be_modified(action_takers)
    }

    /// Sets the token description.
    fn set_description(&mut self, description: Option<String>) {
        self.as_v0_mut().set_description(description)
    }
}

impl TokenConfiguration {
    /// The V0 part of the configuration, whichever version wraps it.
    pub fn as_v0(&self) -> &TokenConfigurationV0 {
        match self {
            TokenConfiguration::V0(v0) => v0,
            TokenConfiguration::V1(v1) => &v1.base,
        }
    }

    /// The V0 part of the configuration, mutably, whichever version wraps it.
    pub fn as_v0_mut(&mut self) -> &mut TokenConfigurationV0 {
        match self {
            TokenConfiguration::V0(v0) => v0,
            TokenConfiguration::V1(v1) => &mut v1.base,
        }
    }
}

impl TokenConfigurationV1Getters for TokenConfiguration {
    fn has_shielded_pool(&self) -> bool {
        match self {
            TokenConfiguration::V0(_) => false,
            TokenConfiguration::V1(v1) => v1.has_shielded_pool(),
        }
    }
}

impl TokenConfigurationV1Setters for TokenConfiguration {
    /// The representation stays canonical: enabling the pool on a V0 configuration upgrades
    /// it in place to V1, and disabling it on a V1 configuration downgrades it back to V0, so
    /// a configuration without a pool always serializes as format version 0 and is accepted
    /// by every protocol version.
    fn set_has_shielded_pool(&mut self, has_shielded_pool: bool) {
        match self {
            TokenConfiguration::V0(v0) => {
                if has_shielded_pool {
                    let base =
                        std::mem::replace(v0, TokenConfigurationV0::default_most_restrictive());
                    *self = TokenConfiguration::V1(TokenConfigurationV1::from_v0(base, true));
                }
            }
            TokenConfiguration::V1(v1) => {
                if has_shielded_pool {
                    v1.set_has_shielded_pool(true);
                } else {
                    let base = std::mem::replace(
                        &mut v1.base,
                        TokenConfigurationV0::default_most_restrictive(),
                    );
                    *self = TokenConfiguration::V0(base);
                }
            }
        }
    }
}
