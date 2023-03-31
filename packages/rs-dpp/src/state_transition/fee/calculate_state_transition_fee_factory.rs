use crate::{state_transition::{
    fee::calculate_state_transition_fee_from_operations_factory::calculate_state_transition_fee_from_operations,
    StateTransition, StateTransitionLike,
}, NonConsensusError};

use super::FeeResult;

pub fn calculate_state_transition_fee(state_transition: &StateTransition) -> Result<FeeResult, NonConsensusError> {
    let execution_context = state_transition.get_execution_context();

    calculate_state_transition_fee_from_operations(
        &execution_context.get_operations(),
        state_transition.get_owner_id(),
    )
}
