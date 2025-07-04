use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::IdentityTokenAccountAlreadyFrozenError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_freeze_transition_action::{TokenFreezeTransitionAction, TokenFreezeTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;
use crate::platform_types::platform::PlatformStateRef;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::group::ModificationOfGroupActionMainParametersNotPermittedError;
use dpp::consensus::state::identity::identity_to_freeze_does_not_exist_error::IdentityToFreezeDoesNotExistError;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::GroupActionAccessors;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::tokens::token_event::TokenEvent;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenFreezeTransitionActionStateValidationV0 {
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

impl TokenFreezeTransitionActionStateValidationV0 for TokenFreezeTransitionAction {
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
        let rules = token_configuration.freeze_rules();
        let main_control_group = token_configuration.main_control_group();
        let validation_result = self.base().validate_group_action(
            rules,
            owner_id,
            contract.owner_id(),
            main_control_group,
            contract.groups(),
            "freeze".to_string(),
            token_configuration,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if let Some(original_group_action) = self.base().original_group_action() {
            // we shouldn't compare the amount, because that is figured out at the end
            if let GroupActionEvent::TokenEvent(TokenEvent::Freeze(identifier, _)) =
                original_group_action.event()
            {
                let mut changed_internal_fields = vec![];
                if identifier != &self.identity_to_freeze_id() {
                    changed_internal_fields.push("identity_to_freeze_id".to_string());
                }
                if !changed_internal_fields.is_empty() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(
                            StateError::ModificationOfGroupActionMainParametersNotPermittedError(
                                ModificationOfGroupActionMainParametersNotPermittedError::new(
                                    original_group_action.event().event_name(),
                                    "Token: freeze".to_string(),
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
                                "Token: freeze".to_string(),
                                vec![],
                            ),
                        ),
                    ),
                ));
            }
        }

        // Check if the identity is already frozen
        let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
            self.token_id().to_buffer(),
            self.identity_to_freeze_id().to_buffer(),
            block_info,
            true,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));
        if let Some(info) = info {
            if info.frozen() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::IdentityTokenAccountAlreadyFrozenError(
                        IdentityTokenAccountAlreadyFrozenError::new(
                            self.token_id(),
                            owner_id,
                            "Freeze Identity Token Account".to_string(),
                        ),
                    )),
                ));
            }
        } else {
            //make sure the identity to freeze exists if we didn't find info
            let recipient_exists = validate_identity_exists(
                platform.drive,
                &self.identity_to_freeze_id(),
                execution_context,
                transaction,
                platform_version,
            )?;

            if !recipient_exists {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::IdentityToFreezeDoesNotExistError(
                        IdentityToFreezeDoesNotExistError::new(self.identity_to_freeze_id()),
                    )),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
