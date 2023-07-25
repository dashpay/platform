use crate::{
    state_transition::{
        fee::calculate_state_transition_fee_from_operations_factory::calculate_state_transition_fee_from_operations,
        StateTransition,
    },
    NonConsensusError,
};

use super::FeeResult;

pub fn calculate_state_transition_fee(
    state_transition: &StateTransition,
    execution_context: &StateTransitionExecutionContext,
) -> Result<FeeResult, NonConsensusError> {
    calculate_state_transition_fee_from_operations(
        &execution_context.get_operations(),
        state_transition.get_owner_id(),
    )
}
