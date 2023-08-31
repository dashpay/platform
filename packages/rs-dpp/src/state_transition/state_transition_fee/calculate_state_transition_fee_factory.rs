use crate::{
    state_transition::{
        StateTransition,
    },
    NonConsensusError,
};
use crate::fee::fee_result::FeeResult;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::state_transition_fee::calculate_state_transition_fee_from_operations_factory::calculate_state_transition_fee_from_operations;


pub fn calculate_state_transition_fee(
    state_transition: &StateTransition,
    execution_context: &StateTransitionExecutionContext,
) -> Result<FeeResult, NonConsensusError> {
    calculate_state_transition_fee_from_operations(
        &execution_context.get_operations(),
        state_transition.owner_id(),
    )
}
