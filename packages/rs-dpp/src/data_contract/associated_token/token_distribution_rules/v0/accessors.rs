use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::{
    TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters,
};
use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::ChangeControlRules;
use platform_value::Identifier;
/// Implementing `TokenDistributionRulesV0Getters` for `TokenDistributionRulesV0`
impl TokenDistributionRulesV0Getters for TokenDistributionRulesV0 {
    /// Returns the perpetual distribution configuration.
    fn perpetual_distribution(&self) -> Option<&TokenPerpetualDistribution> {
        self.perpetual_distribution.as_ref()
    }

    /// Returns the perpetual distribution configuration (mutable).
    fn perpetual_distribution_mut(&mut self) -> Option<&mut TokenPerpetualDistribution> {
        self.perpetual_distribution.as_mut()
    }

    /// Returns the perpetual distribution change rules.
    fn perpetual_distribution_rules(&self) -> &ChangeControlRules {
        &self.perpetual_distribution_rules
    }

    /// Returns the perpetual distribution change rules (mutable).
    fn perpetual_distribution_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.perpetual_distribution_rules
    }

    /// Returns the pre-programmed distribution configuration.
    fn pre_programmed_distribution(&self) -> Option<&TokenPreProgrammedDistribution> {
        self.pre_programmed_distribution.as_ref()
    }

    /// Returns the pre-programmed distribution configuration (mutable).
    fn pre_programmed_distribution_mut(&mut self) -> Option<&mut TokenPreProgrammedDistribution> {
        self.pre_programmed_distribution.as_mut()
    }

    /// Returns the new tokens destination identity.
    fn new_tokens_destination_identity(&self) -> Option<&Identifier> {
        self.new_tokens_destination_identity.as_ref()
    }

    /// Returns the rules for changing the new tokens destination identity.
    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules {
        &self.new_tokens_destination_identity_rules
    }

    /// Returns the rules for changing the new tokens destination identity (mutable).
    fn new_tokens_destination_identity_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.new_tokens_destination_identity_rules
    }

    /// Returns whether minting allows choosing destination.
    fn minting_allow_choosing_destination(&self) -> bool {
        self.minting_allow_choosing_destination
    }

    /// Returns the rules for changing the minting allow choosing destination setting.
    fn minting_allow_choosing_destination_rules(&self) -> &ChangeControlRules {
        &self.minting_allow_choosing_destination_rules
    }

    /// Returns the rules for changing the minting allow choosing destination setting (mutable).
    fn minting_allow_choosing_destination_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.minting_allow_choosing_destination_rules
    }

    fn change_direct_purchase_pricing_rules(&self) -> &ChangeControlRules {
        &self.change_direct_purchase_pricing_rules
    }

    fn change_direct_purchase_pricing_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.change_direct_purchase_pricing_rules
    }
}

/// Implementing `TokenDistributionRulesV0Setters` for `TokenDistributionRulesV0`
impl TokenDistributionRulesV0Setters for TokenDistributionRulesV0 {
    /// Sets the perpetual distribution configuration.
    fn set_perpetual_distribution(
        &mut self,
        perpetual_distribution: Option<TokenPerpetualDistribution>,
    ) {
        self.perpetual_distribution = perpetual_distribution;
    }

    /// Sets the perpetual distribution change rules.
    fn set_perpetual_distribution_rules(&mut self, rules: ChangeControlRules) {
        self.perpetual_distribution_rules = rules;
    }

    /// Sets the pre-programmed distribution configuration.
    fn set_pre_programmed_distribution(
        &mut self,
        pre_programmed_distribution: Option<TokenPreProgrammedDistribution>,
    ) {
        self.pre_programmed_distribution = pre_programmed_distribution;
    }

    /// Sets the new tokens destination identity.
    fn set_new_tokens_destination_identity(&mut self, identity: Option<Identifier>) {
        self.new_tokens_destination_identity = identity;
    }

    /// Sets the rules for changing the new tokens destination identity.
    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules) {
        self.new_tokens_destination_identity_rules = rules;
    }

    /// Sets whether minting allows choosing destination.
    fn set_minting_allow_choosing_destination(&mut self, allow: bool) {
        self.minting_allow_choosing_destination = allow;
    }

    /// Sets the rules for changing the minting allow choosing destination setting.
    fn set_minting_allow_choosing_destination_rules(&mut self, rules: ChangeControlRules) {
        self.minting_allow_choosing_destination_rules = rules;
    }

    fn set_change_direct_purchase_pricing_rules(&mut self, rules: ChangeControlRules) {
        self.change_direct_purchase_pricing_rules = rules;
    }
}
