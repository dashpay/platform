use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::ChangeControlRules;
use platform_value::Identifier;

/// Accessor trait for getters of `TokenDistributionRulesV0`
pub trait TokenDistributionRulesV0Getters {
    /// Returns the perpetual distribution configuration.
    fn perpetual_distribution(&self) -> Option<&TokenPerpetualDistribution>;

    /// Returns the perpetual distribution configuration.
    fn perpetual_distribution_mut(&mut self) -> Option<&mut TokenPerpetualDistribution>;

    /// Returns the perpetual distribution change rules.
    fn perpetual_distribution_rules(&self) -> &ChangeControlRules;

    /// Returns the perpetual distribution change rules.
    fn perpetual_distribution_rules_mut(&mut self) -> &mut ChangeControlRules;

    /// Returns the pre-programmed distribution configuration.
    fn pre_programmed_distribution(&self) -> Option<&TokenPreProgrammedDistribution>;

    /// Returns the pre-programmed distribution configuration.
    fn pre_programmed_distribution_mut(&mut self) -> Option<&mut TokenPreProgrammedDistribution>;

    /// Returns the new tokens destination identity.
    fn new_tokens_destination_identity(&self) -> Option<&Identifier>;

    /// Returns the rules for changing the new tokens destination identity.
    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules;

    /// Returns the rules for changing the new tokens destination identity.
    fn new_tokens_destination_identity_rules_mut(&mut self) -> &mut ChangeControlRules;

    /// Returns whether minting allows choosing destination.
    fn minting_allow_choosing_destination(&self) -> bool;

    /// Returns the rules for changing the minting allow choosing destination setting.
    fn minting_allow_choosing_destination_rules(&self) -> &ChangeControlRules;

    /// Returns the rules for changing the minting allow choosing destination setting.
    fn minting_allow_choosing_destination_rules_mut(&mut self) -> &mut ChangeControlRules;

    /// Returns the rules for changing the direct purchase pricing.
    fn change_direct_purchase_pricing_rules(&self) -> &ChangeControlRules;

    /// Returns the rules for changing the direct purchase pricing as mut.
    fn change_direct_purchase_pricing_rules_mut(&mut self) -> &mut ChangeControlRules;
}

/// Accessor trait for setters of `TokenDistributionRulesV0`
pub trait TokenDistributionRulesV0Setters {
    /// Sets the perpetual distribution configuration.
    fn set_perpetual_distribution(
        &mut self,
        perpetual_distribution: Option<TokenPerpetualDistribution>,
    );

    /// Sets the perpetual distribution change rules.
    fn set_perpetual_distribution_rules(&mut self, rules: ChangeControlRules);

    /// Sets the pre-programmed distribution configuration.
    fn set_pre_programmed_distribution(
        &mut self,
        pre_programmed_distribution: Option<TokenPreProgrammedDistribution>,
    );

    /// Sets the new tokens destination identity.
    fn set_new_tokens_destination_identity(&mut self, identity: Option<Identifier>);

    /// Sets the rules for changing the new tokens destination identity.
    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules);

    /// Sets whether minting allows choosing destination.
    fn set_minting_allow_choosing_destination(&mut self, allow: bool);

    /// Sets the rules for changing the minting allow choosing destination setting.
    fn set_minting_allow_choosing_destination_rules(&mut self, rules: ChangeControlRules);

    /// Sets the rules for changing the direct purchase pricing as mut.
    fn set_change_direct_purchase_pricing_rules(&mut self, rules: ChangeControlRules);
}
