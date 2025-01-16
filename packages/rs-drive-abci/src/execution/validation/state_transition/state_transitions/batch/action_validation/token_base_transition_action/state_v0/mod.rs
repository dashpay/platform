use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::group::GroupActionAlreadySignedByIdentityError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::platform_types::platform::PlatformStateRef;

pub(super) trait TokenBaseTransitionActionStateValidationV0 {
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
impl TokenBaseTransitionActionStateValidationV0 for TokenBaseTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // We should start by validating that if we did not yet sign
        if let Some(group_state_transition_resolved_info) = self.store_in_group() {
            let (already_signed, cost) = platform.drive.fetch_action_id_has_signer_with_costs(
                self.data_contract_id(),
                group_state_transition_resolved_info.group_contract_position,
                group_state_transition_resolved_info.action_id,
                owner_id,
                block_info,
                transaction,
                platform_version,
            )?;
            execution_context.add_operation(ValidationOperation::PrecalculatedOperation(cost));
            if already_signed {
                // We already have signed this state transition group action
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(
                        StateError::GroupActionAlreadySignedByIdentityError(
                            GroupActionAlreadySignedByIdentityError::new(
                                owner_id,
                                self.data_contract_id(),
                                group_state_transition_resolved_info.group_contract_position,
                                group_state_transition_resolved_info.action_id,
                            ),
                        ),
                    ),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
