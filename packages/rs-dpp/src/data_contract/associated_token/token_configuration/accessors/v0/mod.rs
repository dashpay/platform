use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::GroupContractPosition;
use std::collections::BTreeSet;

/// Accessor trait for getters of `TokenConfigurationV0`
pub trait TokenConfigurationV0Getters {
    /// Returns a reference to the conventions.
    fn conventions(&self) -> &TokenConfigurationConvention;

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConvention;
    /// Returns the new tokens destination identity rules.
    fn conventions_change_rules(&self) -> &ChangeControlRules;

    /// Returns the base supply.
    fn base_supply(&self) -> TokenAmount;
    /// Returns the base supply.
    fn keeps_history(&self) -> bool;
    fn start_as_paused(&self) -> bool;

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<TokenAmount>;

    /// Returns the max supply change rules.
    fn max_supply_change_rules(&self) -> &ChangeControlRules;

    /// Returns the distribution rules.
    fn distribution_rules(&self) -> &TokenDistributionRules;

    /// Returns a mutable reference to the distribution rules.
    fn distribution_rules_mut(&mut self) -> &mut TokenDistributionRules;

    /// Returns the manual minting rules.
    fn manual_minting_rules(&self) -> &ChangeControlRules;

    /// Returns the manual burning rules.
    fn manual_burning_rules(&self) -> &ChangeControlRules;

    /// Returns the freeze rules.
    fn freeze_rules(&self) -> &ChangeControlRules;

    /// Returns the unfreeze rules.
    fn unfreeze_rules(&self) -> &ChangeControlRules;
    /// Returns the destroy frozen funds rules.
    fn destroy_frozen_funds_rules(&self) -> &ChangeControlRules;
    /// Returns the emergency action rules.
    fn emergency_action_rules(&self) -> &ChangeControlRules;

    /// Returns the main control group.
    fn main_control_group(&self) -> Option<GroupContractPosition>;

    /// Returns the main control group can be modified.
    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers;

    /// Returns all group positions used in the token configuration
    fn all_used_group_positions(&self) -> BTreeSet<GroupContractPosition>;
}

/// Accessor trait for setters of `TokenConfigurationV0`
pub trait TokenConfigurationV0Setters {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConvention);

    /// Sets the conventions change rules.
    fn set_conventions_change_rules(&mut self, rules: ChangeControlRules);

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: TokenAmount);

    /// Sets the maximum supply.
    fn set_max_supply(&mut self, max_supply: Option<TokenAmount>);

    /// Sets the max supply change rules.
    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules);

    /// Sets the distribution rules.
    fn set_distribution_rules(&mut self, rules: TokenDistributionRules);

    /// Sets the manual minting rules.
    fn set_manual_minting_rules(&mut self, rules: ChangeControlRules);

    /// Sets the manual burning rules.
    fn set_manual_burning_rules(&mut self, rules: ChangeControlRules);

    /// Sets the freeze rules.
    fn set_freeze_rules(&mut self, rules: ChangeControlRules);

    /// Sets the unfreeze rules.
    fn set_unfreeze_rules(&mut self, rules: ChangeControlRules);
    /// Sets the `destroy frozen funds` rules.
    fn set_destroy_frozen_funds_rules(&mut self, rules: ChangeControlRules);
    /// Sets the emergency action rules.
    fn set_emergency_action_rules(&mut self, rules: ChangeControlRules);

    /// Sets the main control group.
    fn set_main_control_group(&mut self, group: Option<GroupContractPosition>);

    /// Sets the main control group can be modified.
    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers);
}
