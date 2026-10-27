use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::{
    TokenDistributionRulesV0Getters, TokenDistributionRulesV0Setters,
};
use crate::data_contract::associated_token::token_distribution_rules::accessors::v1::{
    TokenDistributionRulesV1Getters, TokenDistributionRulesV1Setters,
};
use crate::data_contract::associated_token::token_distribution_rules::v1::TokenDistributionRulesV1;
use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::ChangeControlRules;
use platform_value::Identifier;

impl TokenDistributionRulesV0Getters for TokenDistributionRulesV1 {
    fn perpetual_distribution(&self) -> Option<&TokenPerpetualDistribution> {
        self.perpetual_distribution.as_ref()
    }

    fn perpetual_distribution_mut(&mut self) -> Option<&mut TokenPerpetualDistribution> {
        self.perpetual_distribution.as_mut()
    }

    fn perpetual_distribution_rules(&self) -> &ChangeControlRules {
        &self.perpetual_distribution_rules
    }

    fn perpetual_distribution_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.perpetual_distribution_rules
    }

    fn pre_programmed_distribution(&self) -> Option<&TokenPreProgrammedDistribution> {
        self.pre_programmed_distribution.as_ref()
    }

    fn pre_programmed_distribution_mut(&mut self) -> Option<&mut TokenPreProgrammedDistribution> {
        self.pre_programmed_distribution.as_mut()
    }

    fn new_tokens_destination_identity(&self) -> Option<&Identifier> {
        self.new_tokens_destination_identity.as_ref()
    }

    fn new_tokens_destination_identity_rules(&self) -> &ChangeControlRules {
        &self.new_tokens_destination_identity_rules
    }

    fn new_tokens_destination_identity_rules_mut(&mut self) -> &mut ChangeControlRules {
        &mut self.new_tokens_destination_identity_rules
    }

    fn minting_allow_choosing_destination(&self) -> bool {
        self.minting_allow_choosing_destination
    }

    fn minting_allow_choosing_destination_rules(&self) -> &ChangeControlRules {
        &self.minting_allow_choosing_destination_rules
    }

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

impl TokenDistributionRulesV0Setters for TokenDistributionRulesV1 {
    fn set_perpetual_distribution(
        &mut self,
        perpetual_distribution: Option<TokenPerpetualDistribution>,
    ) {
        self.perpetual_distribution = perpetual_distribution;
    }

    fn set_perpetual_distribution_rules(&mut self, rules: ChangeControlRules) {
        self.perpetual_distribution_rules = rules;
    }

    fn set_pre_programmed_distribution(
        &mut self,
        pre_programmed_distribution: Option<TokenPreProgrammedDistribution>,
    ) {
        self.pre_programmed_distribution = pre_programmed_distribution;
    }

    fn set_new_tokens_destination_identity(&mut self, identity: Option<Identifier>) {
        self.new_tokens_destination_identity = identity;
    }

    fn set_new_tokens_destination_identity_rules(&mut self, rules: ChangeControlRules) {
        self.new_tokens_destination_identity_rules = rules;
    }

    fn set_minting_allow_choosing_destination(&mut self, allow: bool) {
        self.minting_allow_choosing_destination = allow;
    }

    fn set_minting_allow_choosing_destination_rules(&mut self, rules: ChangeControlRules) {
        self.minting_allow_choosing_destination_rules = rules;
    }

    fn set_change_direct_purchase_pricing_rules(&mut self, rules: ChangeControlRules) {
        self.change_direct_purchase_pricing_rules = rules;
    }
}

impl TokenDistributionRulesV1Getters for TokenDistributionRulesV1 {
    fn once_per_identity_distribution(&self) -> Option<&TokenOncePerIdentityDistribution> {
        self.once_per_identity_distribution.as_ref()
    }

    fn once_per_identity_distribution_mut(
        &mut self,
    ) -> Option<&mut TokenOncePerIdentityDistribution> {
        self.once_per_identity_distribution.as_mut()
    }
}

impl TokenDistributionRulesV1Setters for TokenDistributionRulesV1 {
    fn set_once_per_identity_distribution(
        &mut self,
        once_per_identity_distribution: Option<TokenOncePerIdentityDistribution>,
    ) {
        self.once_per_identity_distribution = once_per_identity_distribution;
    }
}
