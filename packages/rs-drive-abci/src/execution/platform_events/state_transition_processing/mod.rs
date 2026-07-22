mod cleanup_recent_block_storage_address_balances;
mod decode_raw_state_transitions;
mod execute_event;
mod process_raw_state_transitions;
mod process_validation_result;
mod record_added_balance_outputs;
mod store_address_balances_to_recent_block_storage;
mod validate_fees_of_event;

use crate::error::Error;

/// Pairs an [`Error`] with the raw state transition (and its name) that produced it, so failures can
/// be logged with context and mapped to an `InternalError` execution result.
///
/// Shared by the `process_raw_state_transitions` outer loop and the `process_validation_result`
/// versioned helper it dispatches to (both are descendants of this module, so the private fields are
/// visible to them).
#[derive(Debug)]
pub(in crate::execution) struct StateTransitionAwareError<'t> {
    error: Error,
    raw_state_transition: &'t [u8],
    state_transition_name: Option<String>,
}
