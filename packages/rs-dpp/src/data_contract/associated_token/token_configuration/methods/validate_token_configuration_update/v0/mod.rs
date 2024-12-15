use crate::consensus::basic::data_contract::DataContractTokenConfigurationUpdateError;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::multi_identity_events::ActionTaker;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;

impl TokenConfiguration {
    #[inline(always)]
    pub(super) fn validate_token_config_update_v0(
        &self,
        new_config: &TokenConfiguration,
        contract_owner_id: &Identifier,
        main_group: &Group,
        action_taker: &ActionTaker,
    ) -> SimpleConsensusValidationResult {
        let old = self.as_cow_v0();
        let new = new_config.as_cow_v0();

        // Check immutable fields: conventions
        if old.conventions != new.conventions {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractTokenConfigurationUpdateError::new(
                    "update".to_string(),
                    "conventions".to_string(),
                    self.clone(),
                    new_config.clone(),
                )
                .into(),
            );
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
        if old.max_supply != new.max_supply
            || old.max_supply_change_rules != new.max_supply_change_rules
        {
            if !old.max_supply_change_rules.can_change_to(
                &new.max_supply_change_rules,
                contract_owner_id,
                main_group,
                action_taker,
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
        if old.new_tokens_destination_identity != new.new_tokens_destination_identity
            || old.new_tokens_destination_identity_rules
                != new.new_tokens_destination_identity_rules
        {
            if !old.new_tokens_destination_identity_rules.can_change_to(
                &new.new_tokens_destination_identity_rules,
                contract_owner_id,
                main_group,
                action_taker,
            ) {
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

        // Check changes to manual_minting_rules
        if old.manual_minting_rules != new.manual_minting_rules {
            if !old.manual_minting_rules.can_change_to(
                &new.manual_minting_rules,
                contract_owner_id,
                main_group,
                action_taker,
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
        if old.manual_burning_rules != new.manual_burning_rules {
            if !old.manual_burning_rules.can_change_to(
                &new.manual_burning_rules,
                contract_owner_id,
                main_group,
                action_taker,
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

        // Check changes to main_control_group
        if old.main_control_group != new.main_control_group {
            if !old
                .main_control_group_can_be_modified
                .allowed_for_action_taker(contract_owner_id, main_group, action_taker)
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
