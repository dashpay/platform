use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::multi_identity_events::ActionTaker;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;

impl TokenConfiguration {
    #[inline(always)]
    pub(super) fn validate_token_config_update_v0(
        &self,
        new_config: &TokenConfiguration,
        contract_id: Identifier,
        action_taker: ActionTaker,
    ) -> SimpleConsensusValidationResult {
        let TokenConfigurationV0 {
            base_supply: max_supply,
            manual_minting_rules: manual_supply_increase_rules,
            main_control_group,
            main_control_group_can_be_modified,
            balance_can_be_increased,
            balance_can_be_destroyed,
        } = self.as_cow_v0();

        let TokenConfigurationV0 {
            base_supply: new_max_supply,
            manual_minting_rules: manual_supply_increase_rules,
            main_control_group,
            main_control_group_can_be_modified,
            balance_can_be_increased: new_balance_can_be_increased,
            balance_can_be_destroyed: new_balance_can_be_destroyed,
        } = new_config.as_cow_v0();

        if max_supply != new_max_supply {
            // max_supply_can_be_increased
        }

        SimpleConsensusValidationResult::new()
    }
}
