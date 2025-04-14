use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::{
    TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters,
};
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::ChangeControlRules;
use platform_value::Identifier;

pub mod v0;
/// Implementing `TokenDistributionRulesV0Getters` for `TokenDistributionRules`
impl TokenDistributionRulesV0Getters for TokenDistributionRules {
    /// Returns the perpetual distribution configuration.
    fn perpetual_distribution(&self) -> Option<&TokenPerpetualDistribution> {
        match self {
            TokenDistributionRules::V0(v0) => v0.perpetual_distribution(),
        }
    }

    /// Returns the perpetual distribution configuration (mutable).
    fn perpetual_distribution_mut(&mut self) -> Option<&mut TokenPerpetualDistribution> {
        match self {
            TokenDistributionRules::V0(v0) => v0.perpetual_distribution_mut(),
        }
    }

    /// Returns the perpetual distribution change rules.
    fn perpetual_distribution_rules(&self) -> &ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.perpetual_distribution_rules(),
        }
    }

    /// Returns the perpetual distribution change rules (mutable).
    fn perpetual_distribution_rules_mut(&mut self) -> &mut ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.perpetual_distribution_rules_mut(),
        }
    }

    /// Returns the pre-programmed distribution configuration.
    fn pre_programmed_distribution(&self) -> Option<&TokenPreProgrammedDistribution> {
        match self {
            TokenDistributionRules::V0(v0) => v0.pre_programmed_distribution(),
        }
    }

    /// Returns the pre-programmed distribution configuration (mutable).
    fn pre_programmed_distribution_mut(&mut self) -> Option<&mut TokenPreProgrammedDistribution> {
        match self {
            TokenDistributionRules::V0(v0) => v0.pre_programmed_distribution_mut(),
        }
    }

    /// Returns the new tokens destination identity.
    fn new_tokens_destination_identity(&self) -> Option<&Identifier> {
        match self {
            TokenDistributionRules::V0(v0) => v0.new_tokens_destination_identity(),
        }
    }

    /// Returns the rules for changing the new tokens destination identity.
    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.new_tokens_destination_identity_rules(),
        }
    }

    /// Returns the rules for changing the new tokens destination identity (mutable).
    fn new_tokens_destination_identity_rules_mut(&mut self) -> &mut ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.new_tokens_destination_identity_rules_mut(),
        }
    }

    /// Returns whether minting allows choosing destination.
    fn minting_allow_choosing_destination(&self) -> bool {
        match self {
            TokenDistributionRules::V0(v0) => v0.minting_allow_choosing_destination(),
        }
    }

    /// Returns the rules for changing the minting allow choosing destination setting.
    fn minting_allow_choosing_destination_rules(&self) -> &ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.minting_allow_choosing_destination_rules(),
        }
    }

    /// Returns the rules for changing the minting allow choosing destination setting (mutable).
    fn minting_allow_choosing_destination_rules_mut(&mut self) -> &mut ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.minting_allow_choosing_destination_rules_mut(),
        }
    }

    fn change_direct_purchase_pricing_rules(&self) -> &ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.change_direct_purchase_pricing_rules(),
        }
    }

    fn change_direct_purchase_pricing_rules_mut(&mut self) -> &mut ChangeControlRules {
        match self {
            TokenDistributionRules::V0(v0) => v0.change_direct_purchase_pricing_rules_mut(),
        }
    }
}

/// Implementing `TokenDistributionRulesV0Setters` for `TokenDistributionRules`
impl TokenDistributionRulesV0Setters for TokenDistributionRules {
    /// Sets the perpetual distribution configuration.
    fn set_perpetual_distribution(
        &mut self,
        perpetual_distribution: Option<TokenPerpetualDistribution>,
    ) {
        match self {
            TokenDistributionRules::V0(v0) => v0.set_perpetual_distribution(perpetual_distribution),
        }
    }

    /// Sets the perpetual distribution change rules.
    fn set_perpetual_distribution_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenDistributionRules::V0(v0) => v0.set_perpetual_distribution_rules(rules),
        }
    }

    /// Sets the pre-programmed distribution configuration.
    fn set_pre_programmed_distribution(
        &mut self,
        pre_programmed_distribution: Option<TokenPreProgrammedDistribution>,
    ) {
        match self {
            TokenDistributionRules::V0(v0) => {
                v0.set_pre_programmed_distribution(pre_programmed_distribution)
            }
        }
    }

    /// Sets the new tokens destination identity.
    fn set_new_tokens_destination_identity(&mut self, identity: Option<Identifier>) {
        match self {
            TokenDistributionRules::V0(v0) => v0.set_new_tokens_destination_identity(identity),
        }
    }

    /// Sets the rules for changing the new tokens destination identity.
    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenDistributionRules::V0(v0) => v0.set_new_tokens_destination_identity_rules(rules),
        }
    }

    /// Sets whether minting allows choosing destination.
    fn set_minting_allow_choosing_destination(&mut self, allow: bool) {
        match self {
            TokenDistributionRules::V0(v0) => v0.set_minting_allow_choosing_destination(allow),
        }
    }

    /// Sets the rules for changing the minting allow choosing destination setting.
    fn set_minting_allow_choosing_destination_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenDistributionRules::V0(v0) => {
                v0.set_minting_allow_choosing_destination_rules(rules)
            }
        }
    }

    fn set_change_direct_purchase_pricing_rules(&mut self, rules: ChangeControlRules) {
        match self {
            TokenDistributionRules::V0(v0) => v0.set_change_direct_purchase_pricing_rules(rules),
        }
    }
}
