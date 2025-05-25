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
