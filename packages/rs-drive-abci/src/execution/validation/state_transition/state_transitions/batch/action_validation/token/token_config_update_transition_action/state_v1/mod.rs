use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::UnauthorizedTokenActionError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::GroupContractPosition;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{
    TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_config_update_transition_action::state_v0::TokenConfigUpdateTransitionActionStateValidationV0;
use crate::platform_types::platform::PlatformStateRef;

fn expected_group_for_action_takers(
    action_takers: AuthorizedActionTakers,
    main_control_group: Option<GroupContractPosition>,
) -> Option<GroupContractPosition> {
    match action_takers {
        AuthorizedActionTakers::MainGroup => main_control_group,
        AuthorizedActionTakers::Group(group_contract_position) => Some(group_contract_position),
        AuthorizedActionTakers::NoOne
        | AuthorizedActionTakers::ContractOwner
        | AuthorizedActionTakers::Identity(_) => None,
    }
}

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenConfigUpdateTransitionActionStateValidationV1 {
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenConfigUpdateTransitionActionStateValidationV1 for TokenConfigUpdateTransitionAction {
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let main_control_group = token_configuration.main_control_group();
        let authorized_action_takers = token_configuration
            .controlling_action_takers_for_configuration_item(
                self.update_token_configuration_item(),
            );

        if let Some(resolved_group_info) = self.base().store_in_group() {
            let expected_group =
                expected_group_for_action_takers(authorized_action_takers, main_control_group);

            if expected_group != Some(resolved_group_info.group_contract_position) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::UnauthorizedTokenActionError(
                        UnauthorizedTokenActionError::new(
                            self.token_id(),
                            owner_id,
                            "config_update".to_string(),
                            authorized_action_takers,
                        ),
                    )),
                ));
            }
        }

        // Pass the controlling rule so an unauthorized `MainControlGroup` change item
        // is reported with `main_control_group_can_be_modified` instead of the
        // hardcoded `NoOne` that v0 reports.
        let validation_result = self.validate_state_v0_with_reported_action_takers(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
            Some(authorized_action_takers),
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let mut proposed_token_configuration = token_configuration.clone();
        proposed_token_configuration
            .apply_token_configuration_item(self.update_token_configuration_item().clone());

        let validation_result = proposed_token_configuration
            .validate_token_config_groups_exist(contract.groups(), platform_version)?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_group_comes_only_from_current_authority() {
        assert_eq!(
            expected_group_for_action_takers(AuthorizedActionTakers::MainGroup, Some(4)),
            Some(4)
        );
        assert_eq!(
            expected_group_for_action_takers(AuthorizedActionTakers::MainGroup, None),
            None
        );
        assert_eq!(
            expected_group_for_action_takers(AuthorizedActionTakers::Group(9), Some(4)),
            Some(9)
        );
        assert_eq!(
            expected_group_for_action_takers(AuthorizedActionTakers::ContractOwner, Some(4)),
            None
        );
    }
}
