use crate::consensus::basic::data_contract::{
    GroupPositionDoesNotExistError, MainGroupIsNotDefinedError,
};
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::GroupContractPosition;
use crate::validation::SimpleConsensusValidationResult;
use std::collections::BTreeMap;

impl TokenConfiguration {
    #[inline(always)]
    pub(super) fn validate_token_config_groups_exist_v0(
        &self,
        groups: &BTreeMap<GroupContractPosition, Group>,
    ) -> SimpleConsensusValidationResult {
        // Initialize validation result
        let validation_result = SimpleConsensusValidationResult::new();

        // Collect all group positions used in the token configuration
        let (group_positions, uses_main_group) = self.all_used_group_positions();

        // Check that all referenced group positions exist in the provided groups map
        for group_position in group_positions {
            if !groups.contains_key(&group_position) {
                return SimpleConsensusValidationResult::new_with_error(
                    GroupPositionDoesNotExistError::new(group_position).into(),
                );
            }
        }

        if uses_main_group && self.main_control_group().is_none() {
            return SimpleConsensusValidationResult::new_with_error(
                MainGroupIsNotDefinedError::new().into(),
            );
        }

        // If a main group is defined in the token configuration, verify its existence
        if let Some(main_group_position) = self.main_control_group() {
            if !groups.contains_key(&main_group_position) {
                return SimpleConsensusValidationResult::new_with_error(
                    GroupPositionDoesNotExistError::new(main_group_position).into(),
                );
            }
        }

        // If we reach here with no errors, return an empty result
        validation_result
    }
}
