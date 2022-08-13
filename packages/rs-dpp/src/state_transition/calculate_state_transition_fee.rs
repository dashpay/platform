use crate::prelude::ProtocolError;

use super::StateTransitionLike;

pub const PRICE_PER_BYTE: u64 = 1;

/**
 * Get State Transition fee size
 *
 * @typedef calculateStateTransitionFee
 * @param { DataContractCreateTransition|
 * DocumentsBatchTransition|
 * IdentityCreateTransition} stateTransition
 * @return {number}
 */
pub fn calculate_state_transition_fee(
    state_transition: &impl StateTransitionLike,
) -> Result<u64, ProtocolError> {
    let serialized_state_transition = state_transition.to_buffer(true)?;
    Ok(serialized_state_transition.len() as u64 * PRICE_PER_BYTE)
}
