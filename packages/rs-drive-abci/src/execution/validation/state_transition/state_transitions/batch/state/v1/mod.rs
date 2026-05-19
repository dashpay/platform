use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::BatchTransition;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformStateRef;

/// PROTOCOL_VERSION_12+: like v0, but threads the caller's
/// `execution_context` into `try_into_action_v0` so per-transition
/// fee_results accumulated by the transformer
/// (`try_from_borrowed_*_with_contract_lookup`) are billed to the user
/// instead of being dropped via a local ctx.
///
/// The transformer body (`try_into_action_v0` and all helpers in
/// `transformer/v0/mod.rs`) is intentionally still at `_v0` per the
/// file-header comment there — both `transform_into_action_v0` and this
/// `_v1` wrapper share the same single transformer entry point.
pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStateValidationV1
{
    fn transform_into_action_v1(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStateValidationV1 for BatchTransition {
    fn transform_into_action_v1(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let validation_result = self.try_into_action_v0(
            platform,
            block_info,
            validation_mode.should_validate_batch_valid_against_state(),
            tx,
            execution_context,
        )?;

        Ok(validation_result.map(Into::into))
    }
}
