use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{TokenAlreadyPausedError, TokenNotPausedError};
use dpp::consensus::ConsensusError;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::prelude::Identifier;
use dpp::tokens::emergency_action::TokenEmergencyAction;
use dpp::tokens::status::v0::TokenStatusV0Accessors;
use dpp::tokens::token_event::TokenEvent;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_emergency_action_transition_action::{TokenEmergencyActionTransitionAction, TokenEmergencyActionTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenEmergencyActionTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenEmergencyActionTransitionActionStateValidationV0
    for TokenEmergencyActionTransitionAction
{
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_state(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Let's first check to see if we are authorized to perform this action
        let contract = &self.data_contract_fetch_info_ref().contract;
        let token_configuration = contract.expected_token_configuration(self.token_position())?;
        let rules = token_configuration.emergency_action_rules();
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "emergency action".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if let Some(original_group_action) = self.base().original_group_action() {
            // we shouldn't compare the amount, because that is figured out at the end
            if let GroupActionEvent::TokenEvent(TokenEvent::EmergencyAction(action, _)) =
                original_group_action.event()
            {
                let mut changed_internal_fields = vec![];
                if action != &self.emergency_action() {
                    changed_internal_fields.push("emergency_action".to_string());
                }
                if !changed_internal_fields.is_empty() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                                ModificationOfGroupActionMainParametersNotPermittedError::new(
                                    original_group_action.event().event_name(),
                                    "Token: emergencyAction".to_string(),
                                    changed_internal_fields,
                                ),
                            ),
                        ),
                    ));
                }
            } else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                            ModificationOfGroupActionMainParametersNotPermittedError::new(
                                original_group_action.event().event_name(),
                                "Token: emergencyAction".to_string(),
                                vec![],
                            ),
                        ),
                    ),
                ));
            }
        }

        // Check if we are paused
        let (maybe_token_status, fee_result) = platform.drive.fetch_token_status_with_costs(
            self.token_id().to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
        if let Some(token_status) = maybe_token_status {
            match self.emergency_action() {
                TokenEmergencyAction::Pause => {
                    if token_status.paused() {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::TokenAlreadyPausedError(
                                TokenAlreadyPausedError::new(
                                    self.token_id(),
                                    "Pause Token".to_string(),
                                ),
                            )),
                        ));
                    }
                }
                TokenEmergencyAction::Resume => {
                    if !token_status.paused() {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            ConsensusError::StateError(StateError::TokenNotPausedError(
                                TokenNotPausedError::new(
                                    self.token_id(),
                                    "Resume Token".to_string(),
                                ),
                            )),
                        ));
                    }
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
