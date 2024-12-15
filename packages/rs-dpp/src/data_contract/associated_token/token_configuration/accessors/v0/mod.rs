use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationConventionV0;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::group::RequiredSigners;
use platform_value::Identifier;
use std::collections::BTreeSet;

/// Accessor trait for getters of `TokenConfigurationV0`
pub trait TokenConfigurationV0Getters {
    /// Returns a reference to the conventions.
    fn conventions(&self) -> &TokenConfigurationConventionV0;

    /// Returns a mutable reference to the conventions.
    fn conventions_mut(&mut self) -> &mut TokenConfigurationConventionV0;

    /// Returns the base supply.
    fn base_supply(&self) -> u64;

    /// Returns the maximum supply.
    fn max_supply(&self) -> Option<u64>;

    /// Returns the max supply change rules.
    fn max_supply_change_rules(&self) -> &ChangeControlRules;

    /// Returns the new tokens destination identity.
    fn new_tokens_destination_identity(&self) -> Option<Identifier>;

    /// Returns the new tokens destination identity rules.
    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules;

    /// Returns the manual minting rules.
    fn manual_minting_rules(&self) -> &ChangeControlRules;

    /// Returns the manual burning rules.
    fn manual_burning_rules(&self) -> &ChangeControlRules;

    /// Returns the freeze rules.
    fn freeze_rules(&self) -> &ChangeControlRules;

    /// Returns the unfreeze rules.
    fn unfreeze_rules(&self) -> &ChangeControlRules;

    /// Returns the main control group.
    fn main_control_group(&self) -> Option<&(BTreeSet<Identifier>, RequiredSigners)>;

    /// Returns the main control group can be modified.
    fn main_control_group_can_be_modified(&self) -> &AuthorizedActionTakers;
}

/// Accessor trait for setters of `TokenConfigurationV0`
pub trait TokenConfigurationV0Setters {
    /// Sets the conventions.
    fn set_conventions(&mut self, conventions: TokenConfigurationConventionV0);

    /// Sets the base supply.
    fn set_base_supply(&mut self, base_supply: u64);

    /// Sets the maximum supply.
    fn set_max_supply(&mut self, max_supply: Option<u64>);

    /// Sets the max supply change rules.
    fn set_max_supply_change_rules(&mut self, rules: ChangeControlRules);

    /// Sets the new tokens destination identity.
    fn set_new_tokens_destination_identity(&mut self, id: Option<Identifier>);

    /// Sets the new tokens destination identity rules.
    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules);

    /// Sets the manual minting rules.
    fn set_manual_minting_rules(&mut self, rules: ChangeControlRules);

    /// Sets the manual burning rules.
    fn set_manual_burning_rules(&mut self, rules: ChangeControlRules);

    /// Sets the freeze rules.
    fn set_freeze_rules(&mut self, rules: ChangeControlRules);

    /// Sets the unfreeze rules.
    fn set_unfreeze_rules(&mut self, rules: ChangeControlRules);

    /// Sets the main control group.
    fn set_main_control_group(&mut self, group: Option<(BTreeSet<Identifier>, RequiredSigners)>);

    /// Sets the main control group can be modified.
    fn set_main_control_group_can_be_modified(&mut self, action_takers: AuthorizedActionTakers);
}
