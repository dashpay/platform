use crate::consensus::basic::data_contract::DataContractTokenConfigurationUpdateError;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_marketplace_rules::accessors::v0::TokenMarketplaceRulesV0Getters;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::group::action_taker::{ActionGoal, ActionTaker};
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;
use std::collections::BTreeMap;

impl TokenConfiguration {
    #[inline(always)]
    pub(super) fn validate_token_config_update_v0(
        &self,
        new_config: &TokenConfiguration,
        contract_owner_id: &Identifier,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_taker: &ActionTaker,
        goal: ActionGoal,
    ) -> SimpleConsensusValidationResult {
        let old = self.as_cow_v0();
        let new = new_config.as_cow_v0();

        // Check immutable fields: conventions
        #[allow(clippy::collapsible_if)]
        if old.conventions != new.conventions
            || old.conventions_change_rules != new.conventions_change_rules
        {
            if !old.conventions_change_rules.can_change_to(
                &new.conventions_change_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "conventions or conventionsRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check immutable fields: base_supply
        if old.base_supply != new.base_supply {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractTokenConfigurationUpdateError::new(
                    "update".to_string(),
                    "baseSupply".to_string(),
                    self.clone(),
                    new_config.clone(),
                )
                .into(),
            );
        }

        // Check changes to max_supply and max_supply_change_rules
        #[allow(clippy::collapsible_if)]
        if old.max_supply != new.max_supply
            || old.max_supply_change_rules != new.max_supply_change_rules
        {
            if !old.max_supply_change_rules.can_change_to(
                &new.max_supply_change_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "maxSupply or maxSupplyChangeRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to new_tokens_destination_identity and rules
        #[allow(clippy::collapsible_if)]
        if old.distribution_rules.new_tokens_destination_identity()
            != new.distribution_rules.new_tokens_destination_identity()
            || old
                .distribution_rules
                .new_tokens_destination_identity_rules()
                != new
                    .distribution_rules
                    .new_tokens_destination_identity_rules()
        {
            if !old
                .distribution_rules
                .new_tokens_destination_identity_rules()
                .can_change_to(
                    new.distribution_rules
                        .new_tokens_destination_identity_rules(),
                    contract_owner_id,
                    self.main_control_group(),
                    groups,
                    action_taker,
                    goal,
                )
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "newTokensDestinationIdentity or newTokensDestinationIdentityRules"
                            .to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to minting_allow_choosing_destination and its rules
        #[allow(clippy::collapsible_if)]
        if old.distribution_rules.minting_allow_choosing_destination()
            != new.distribution_rules.minting_allow_choosing_destination()
            || old
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                != new
                    .distribution_rules
                    .minting_allow_choosing_destination_rules()
        {
            if !old
                .distribution_rules
                .minting_allow_choosing_destination_rules()
                .can_change_to(
                    new.distribution_rules
                        .minting_allow_choosing_destination_rules(),
                    contract_owner_id,
                    self.main_control_group(),
                    groups,
                    action_taker,
                    goal,
                )
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "mintingAllowChoosingDestination or mintingAllowChoosingDestinationRules"
                            .to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to change_direct_purchase_pricing_rules and its rules
        #[allow(clippy::collapsible_if)]
        if old
            .distribution_rules
            .change_direct_purchase_pricing_rules()
            != new
                .distribution_rules
                .change_direct_purchase_pricing_rules()
        {
            if !old
                .distribution_rules
                .change_direct_purchase_pricing_rules()
                .can_change_to(
                    new.distribution_rules
                        .change_direct_purchase_pricing_rules(),
                    contract_owner_id,
                    self.main_control_group(),
                    groups,
                    action_taker,
                    goal,
                )
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "change_direct_purchase_pricing_rules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to marketplace trade mode and its rules
        #[allow(clippy::collapsible_if)]
        if old.marketplace_rules.trade_mode() != new.marketplace_rules.trade_mode()
            || old.marketplace_rules.trade_mode_change_rules()
                != new.marketplace_rules.trade_mode_change_rules()
        {
            if !old
                .marketplace_rules
                .trade_mode_change_rules()
                .can_change_to(
                    new.marketplace_rules.trade_mode_change_rules(),
                    contract_owner_id,
                    self.main_control_group(),
                    groups,
                    action_taker,
                    goal,
                )
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "marketplace_rules trade_mode or marketplace_rules trade_mode_change_rules"
                            .to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to perpetual_distribution and its rules
        #[allow(clippy::collapsible_if)]
        if old.distribution_rules.perpetual_distribution()
            != new.distribution_rules.perpetual_distribution()
            || old.distribution_rules.perpetual_distribution_rules()
                != new.distribution_rules.perpetual_distribution_rules()
        {
            if !old
                .distribution_rules
                .perpetual_distribution_rules()
                .can_change_to(
                    new.distribution_rules.perpetual_distribution_rules(),
                    contract_owner_id,
                    self.main_control_group(),
                    groups,
                    action_taker,
                    goal,
                )
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "perpetualDistribution or perpetualDistributionRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check immutable fields: pre_programmed_distribution
        if old.distribution_rules.pre_programmed_distribution()
            != new.distribution_rules.pre_programmed_distribution()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractTokenConfigurationUpdateError::new(
                    "update".to_string(),
                    "preProgrammedDistribution".to_string(),
                    self.clone(),
                    new_config.clone(),
                )
                .into(),
            );
        }

        // Check changes to manual_minting_rules
        #[allow(clippy::collapsible_if)]
        if old.manual_minting_rules != new.manual_minting_rules {
            if !old.manual_minting_rules.can_change_to(
                &new.manual_minting_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "manualMintingRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to manual_burning_rules
        #[allow(clippy::collapsible_if)]
        if old.manual_burning_rules != new.manual_burning_rules {
            if !old.manual_burning_rules.can_change_to(
                &new.manual_burning_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "manualBurningRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to freeze_rules
        #[allow(clippy::collapsible_if)]
        if old.freeze_rules != new.freeze_rules {
            if !old.freeze_rules.can_change_to(
                &new.freeze_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "freezeRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to unfreeze_rules
        #[allow(clippy::collapsible_if)]
        if old.unfreeze_rules != new.unfreeze_rules {
            if !old.unfreeze_rules.can_change_to(
                &new.unfreeze_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "unfreezeRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to destroy_frozen_funds_rules
        #[allow(clippy::collapsible_if)]
        if old.destroy_frozen_funds_rules != new.destroy_frozen_funds_rules {
            if !old.destroy_frozen_funds_rules.can_change_to(
                &new.destroy_frozen_funds_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "destroyFrozenFundsRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to emergency_action_rules
        #[allow(clippy::collapsible_if)]
        if old.emergency_action_rules != new.emergency_action_rules {
            if !old.emergency_action_rules.can_change_to(
                &new.emergency_action_rules,
                contract_owner_id,
                self.main_control_group(),
                groups,
                action_taker,
                goal,
            ) {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "emergencyActionRules".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to main_control_group
        #[allow(clippy::collapsible_if)]
        if old.main_control_group != new.main_control_group {
            if !old
                .main_control_group_can_be_modified
                .allowed_for_action_taker(
                    contract_owner_id,
                    self.main_control_group(),
                    groups,
                    action_taker,
                    goal,
                )
            {
                return SimpleConsensusValidationResult::new_with_error(
                    DataContractTokenConfigurationUpdateError::new(
                        "update".to_string(),
                        "mainControlGroup".to_string(),
                        self.clone(),
                        new_config.clone(),
                    )
                    .into(),
                );
            }
        }

        // Check changes to main_control_group_can_be_modified
        if old.main_control_group_can_be_modified != new.main_control_group_can_be_modified {
            // Assuming this is immutable
            return SimpleConsensusValidationResult::new_with_error(
                DataContractTokenConfigurationUpdateError::new(
                    "update".to_string(),
                    "mainControlGroupCanBeModified".to_string(),
                    self.clone(),
                    new_config.clone(),
                )
                .into(),
            );
        }

        // If we reach here with no errors, return an empty result
        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
    use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
    use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
    use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
    use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
    use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
    use crate::data_contract::associated_token::token_marketplace_rules::v0::{
        TokenMarketplaceRulesV0, TokenTradeMode,
    };
    use crate::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
    use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::data_contract::change_control_rules::ChangeControlRules;
    use crate::data_contract::group::v0::GroupV0;
    use crate::data_contract::group::Group;
    use crate::group::action_taker::{ActionGoal, ActionTaker};
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    /// Helper to build a default no-one-allowed change control rule.
    fn no_one_rules() -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        })
    }

    /// Helper to build change control rules where the contract owner can make changes.
    fn contract_owner_rules() -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::ContractOwner,
            changing_authorized_action_takers_to_no_one_allowed: true,
            changing_admin_action_takers_to_no_one_allowed: true,
            self_changing_admin_action_takers_allowed: true,
        })
    }

    /// Helper to build a default TokenConfigurationV0 for testing.
    fn default_config() -> TokenConfigurationV0 {
        TokenConfigurationV0 {
            conventions: TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                localizations: Default::default(),
                decimals: 8,
            }),
            conventions_change_rules: no_one_rules(),
            base_supply: 100_000,
            max_supply: None,
            keeps_history: TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
                keeps_transfer_history: true,
                keeps_freezing_history: true,
                keeps_minting_history: true,
                keeps_burning_history: true,
                keeps_direct_pricing_history: true,
                keeps_direct_purchase_history: true,
            }),
            start_as_paused: false,
            allow_transfer_to_frozen_balance: true,
            max_supply_change_rules: no_one_rules(),
            distribution_rules: TokenDistributionRules::V0(TokenDistributionRulesV0 {
                perpetual_distribution: None,
                perpetual_distribution_rules: no_one_rules(),
                pre_programmed_distribution: None,
                new_tokens_destination_identity: None,
                new_tokens_destination_identity_rules: no_one_rules(),
                minting_allow_choosing_destination: true,
                minting_allow_choosing_destination_rules: no_one_rules(),
                change_direct_purchase_pricing_rules: no_one_rules(),
            }),
            marketplace_rules: TokenMarketplaceRules::V0(TokenMarketplaceRulesV0 {
                trade_mode: TokenTradeMode::NotTradeable,
                trade_mode_change_rules: no_one_rules(),
            }),
            manual_minting_rules: contract_owner_rules(),
            manual_burning_rules: contract_owner_rules(),
            freeze_rules: no_one_rules(),
            unfreeze_rules: no_one_rules(),
            destroy_frozen_funds_rules: no_one_rules(),
            emergency_action_rules: no_one_rules(),
            main_control_group: None,
            main_control_group_can_be_modified: AuthorizedActionTakers::NoOne,
            description: None,
        }
    }

    #[test]
    fn identical_configs_should_pass_validation() {
        let owner_id = Identifier::random();
        let config = TokenConfiguration::V0(default_config());
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = config.validate_token_config_update_v0(
            &config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            result.is_valid(),
            "identical configs should pass validation"
        );
    }

    #[test]
    fn changing_base_supply_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.base_supply = 200_000;
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing base_supply should produce an error"
        );
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn changing_conventions_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.conventions = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 6, // changed from 8
        });
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing conventions when no one can should fail"
        );
    }

    #[test]
    fn changing_conventions_with_owner_rules_as_owner_should_pass() {
        let owner_id = Identifier::random();
        let mut old_v0 = default_config();
        old_v0.conventions_change_rules = contract_owner_rules();
        let old_config = TokenConfiguration::V0(old_v0.clone());

        let mut new_v0 = old_v0;
        new_v0.conventions = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 6,
        });
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            result.is_valid(),
            "contract owner should be allowed to change conventions: {:?}",
            result.errors
        );
    }

    #[test]
    fn non_owner_changing_conventions_with_owner_rules_should_fail() {
        let owner_id = Identifier::random();
        let non_owner = Identifier::random();
        let mut old_v0 = default_config();
        old_v0.conventions_change_rules = contract_owner_rules();
        let old_config = TokenConfiguration::V0(old_v0.clone());

        let mut new_v0 = old_v0;
        new_v0.conventions = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 6,
        });
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(non_owner);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "non-owner should not be allowed to change conventions"
        );
    }

    #[test]
    fn changing_max_supply_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.max_supply = Some(500_000);
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing max_supply with NoOne rules should fail"
        );
    }

    #[test]
    fn changing_manual_minting_rules_as_owner_should_pass() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.manual_minting_rules = no_one_rules();
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            result.is_valid(),
            "owner changing manual_minting_rules should pass: {:?}",
            result.errors
        );
    }

    #[test]
    fn changing_manual_burning_rules_as_non_owner_should_fail() {
        let owner_id = Identifier::random();
        let non_owner = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.manual_burning_rules = no_one_rules();
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(non_owner);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "non-owner changing manual_burning_rules should fail"
        );
    }

    #[test]
    fn changing_freeze_rules_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.freeze_rules = contract_owner_rules();
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing freeze_rules with NoOne rules should fail"
        );
    }

    #[test]
    fn changing_unfreeze_rules_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.unfreeze_rules = contract_owner_rules();
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing unfreeze_rules with NoOne rules should fail"
        );
    }

    #[test]
    fn changing_destroy_frozen_funds_rules_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.destroy_frozen_funds_rules = contract_owner_rules();
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing destroy_frozen_funds_rules with NoOne rules should fail"
        );
    }

    #[test]
    fn changing_emergency_action_rules_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());
        let mut new_v0 = default_config();
        new_v0.emergency_action_rules = contract_owner_rules();
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing emergency_action_rules with NoOne rules should fail"
        );
    }

    #[test]
    fn changing_main_control_group_can_be_modified_should_always_fail() {
        let owner_id = Identifier::random();
        let mut old_v0 = default_config();
        old_v0.main_control_group_can_be_modified = AuthorizedActionTakers::ContractOwner;
        let old_config = TokenConfiguration::V0(old_v0);

        let mut new_v0 = default_config();
        new_v0.main_control_group_can_be_modified = AuthorizedActionTakers::NoOne;
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing mainControlGroupCanBeModified should always fail"
        );
    }

    #[test]
    fn changing_main_control_group_as_owner_when_allowed_should_pass() {
        let owner_id = Identifier::random();
        let member_id = Identifier::random();
        let mut members = BTreeMap::new();
        members.insert(member_id, 1u32);
        members.insert(owner_id, 1u32);
        let group = Group::V0(GroupV0 {
            members,
            required_power: 1,
        });
        let mut groups = BTreeMap::new();
        groups.insert(0u16, group);

        let mut old_v0 = default_config();
        old_v0.main_control_group = Some(0u16);
        old_v0.main_control_group_can_be_modified = AuthorizedActionTakers::ContractOwner;
        let old_config = TokenConfiguration::V0(old_v0.clone());

        let mut new_v0 = old_v0;
        new_v0.main_control_group = None;
        let new_config = TokenConfiguration::V0(new_v0);
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            result.is_valid(),
            "owner should be able to change main_control_group when allowed: {:?}",
            result.errors
        );
    }

    #[test]
    fn changing_main_control_group_as_non_owner_when_owner_required_should_fail() {
        let owner_id = Identifier::random();
        let non_owner = Identifier::random();

        let mut old_v0 = default_config();
        old_v0.main_control_group = Some(0u16);
        old_v0.main_control_group_can_be_modified = AuthorizedActionTakers::ContractOwner;
        let old_config = TokenConfiguration::V0(old_v0.clone());

        let mut new_v0 = old_v0;
        new_v0.main_control_group = None;
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(non_owner);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "non-owner should not be able to change main_control_group"
        );
    }

    #[test]
    fn changing_marketplace_trade_mode_change_rules_with_no_one_rules_should_fail() {
        let owner_id = Identifier::random();
        let old_config = TokenConfiguration::V0(default_config());

        let mut new_v0 = default_config();
        // Change the trade_mode_change_rules to something different
        new_v0.marketplace_rules = TokenMarketplaceRules::V0(TokenMarketplaceRulesV0 {
            trade_mode: TokenTradeMode::NotTradeable,
            trade_mode_change_rules: contract_owner_rules(),
        });
        let new_config = TokenConfiguration::V0(new_v0);
        let groups = BTreeMap::new();
        let action_taker = ActionTaker::SingleIdentity(owner_id);

        let result = old_config.validate_token_config_update_v0(
            &new_config,
            &owner_id,
            &groups,
            &action_taker,
            ActionGoal::ActionCompletion,
        );

        assert!(
            !result.is_valid(),
            "changing trade_mode_change_rules with NoOne rules should fail"
        );
    }
}
