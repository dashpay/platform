use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::{
    fee::calculate_state_transition_fee_from_operations_factory::calculate_state_transition_fee_from_operations,
    StateTransition, StateTransitionLike,
};

use super::FeeResult;

pub fn calculate_state_transition_fee(
    state_transition: &StateTransition,
    execution_context: &StateTransitionExecutionContext,
) -> FeeResult {
    calculate_state_transition_fee_from_operations(
        &execution_context.get_operations(),
        state_transition.get_owner_id(),
    )
}
