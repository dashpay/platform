pub const PRICE_PER_BYTE: u64 = 1;
use crate::prelude::ProtocolError;

use super::{StateTransition, StateTransitionConvert};

/**
 * Get State Transition fee size
 *
 * @typedef calculateStateTransitionFee
 * @param { DataContractCreateTransition|
 * DocumentsBatchTransition|
 * IdentityCreateTransition} stateTransition
 * @return {number}
 */
fn calculate_state_transition_fee(state_transition: StateTransition) -> Result<u64, ProtocolError> {
    // TODO  should we allow calculate Fee for DataContractUpdate or  IdentityTopUp
    let serialized_state_transition = state_transition.to_buffer(true)?;
    Ok(serialized_state_transition.len() as u64 * PRICE_PER_BYTE)
}
