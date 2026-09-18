use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v1::BatchTransitionTransformerV1;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::BatchTransition;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

/// Generation 2 of the batch transformation, selected for protocol version 14
/// and later: like generation 1 it threads the caller's execution context into
/// the transformer, and it runs transformer generation 1, which sees every batch
/// wire format and produces batch action format 1.
pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionActionTransformV2
{
    fn transform_into_action_v2(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionActionTransformV2 for BatchTransition {
    fn transform_into_action_v2(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let validation_result = self.try_into_action_v1(
            platform,
            block_info,
            validation_mode.should_validate_batch_valid_against_state(),
            tx,
            execution_context,
        )?;

        Ok(validation_result.map(StateTransitionAction::BatchActionV1))
    }
}
