use crate::error::Error;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::codes::ErrorWithCode;

use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, InvalidStateTransition, InvalidWithProtocolErrorStateTransition,
    SuccessfullyDecodedStateTransition,
};
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::metrics::{state_transition_execution_histogram, HistogramTiming};
use crate::platform_types::state_transitions_processing_result::{
    NotExecutedReason, StateTransitionExecutionResult, StateTransitionsProcessingResult,
};
use dpp::util::hash::hash_single;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use drive::grovedb_storage::Error::RocksDBError;
use std::time::Instant;

use super::super::StateTransitionAwareError;

/// Test-only fault injection: force the next successfully executed state transition to be
/// reported as an `InternalError` AFTER its drive operations were applied. This models the
/// only way an `InternalError` can carry state (an `Err` surfacing after
/// `apply_drive_operations(apply = true)`) without depending on any particular estimation
/// bug, so the rollback below stays pinned even once every known trigger is fixed.
#[cfg(test)]
pub(crate) mod test_fault_injection {
    use std::cell::Cell;

    thread_local! {
        pub static FAIL_NEXT_SUCCESSFUL_EXECUTION: Cell<bool> = const { Cell::new(false) };
    }
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Processes the given raw state transitions based on the `block_info` and `transaction`.
    ///
    /// Differs from v0 in one way: every executed state transition is wrapped in a GroveDB
    /// savepoint, and a result that strips the transition from the block (`InternalError` /
    /// `UnpaidConsensusError`, both mapped to `TxAction::Removed` by `prepare_proposal`) rolls
    /// the savepoint back. Execution can write into the shared block transaction before
    /// failing (the fee flow is apply-then-check), so without the rollback a dropped
    /// transition's writes stay in the transaction and pollute the proposer's app hash while
    /// the gossiped block omits the transition — no validator can then reproduce the hash
    /// (mainnet evo1 stalls of 2026-08-14/15, after heights 415652 and 415661).
    ///
    /// Savepoints of kept transitions are left on the transaction's savepoint stack: RocksDB
    /// has no exposed way to pop one without rolling back, leftover savepoints are inert for
    /// commit, and the per-round transaction they live in is dropped when the round ends. The
    /// one consumer of that stack — the genesis-height re-proposal path — drains the whole
    /// stack rather than popping once, precisely so this residue cannot redirect it.
    ///
    /// # Arguments
    ///
    /// * `raw_state_transitions` - A reference to a vector of raw state transitions.
    /// * `block_info` - Information about the current block being processed.
    /// * `transaction` - The transaction associated with the raw state transitions.
    ///
    /// # Returns
    ///
    /// * `Result<StateTransitionsProcessingResult, Error>` - If the processing is successful, it returns
    ///   a `StateTransitionsProcessingResult` with state transition execution results and aggregated information.
    ///   If the processing fails, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with deserializing the raw
    /// state transitions, processing state transitions, executing events, or rolling back a
    /// dropped transition's savepoint.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_raw_state_transitions_v1(
        &self,
        raw_state_transitions: &[Vec<u8>],
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        proposing_state_transitions: bool,
        timer: Option<&HistogramTiming>,
    ) -> Result<StateTransitionsProcessingResult, Error> {
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: block_platform_state,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };

        let state_transition_container =
            self.decode_raw_state_transitions(raw_state_transitions, platform_version)?;

        let mut processing_result = StateTransitionsProcessingResult::default();

        for decoded_state_transition in state_transition_container.into_iter() {
            // If we propose state transitions, we need to check if we have a time limit for processing
            // set and if we have exceeded it.
            let execution_result = if proposing_state_transitions
                && timer.is_some_and(|timer| {
                    timer.elapsed().as_millis()
                        > self
                            .config
                            .abci
                            .proposer_tx_processing_time_limit
                            .unwrap_or(u16::MAX) as u128
                }) {
                StateTransitionExecutionResult::NotExecuted(NotExecutedReason::ProposerRanOutOfTime)
            } else {
                match decoded_state_transition {
                    DecodedStateTransition::SuccessfullyDecoded(
                        SuccessfullyDecodedStateTransition {
                            decoded: state_transition,
                            raw: raw_state_transition,
                            elapsed_time: decoding_elapsed_time,
                        },
                    ) => {
                        let start_time = Instant::now();

                        let state_transition_name = state_transition.name();

                        if tracing::enabled!(tracing::Level::TRACE) {
                            let st_hash = hex::encode(hash_single(raw_state_transition));

                            tracing::trace!(
                                ?state_transition,
                                st_hash,
                                "Processing {} state transition",
                                state_transition_name
                            );
                        }

                        // Execution may write into the shared block transaction before its
                        // result is known, so mark the state we can return to if the result
                        // strips this transition from the block.
                        transaction.set_savepoint();

                        // Validate state transition and produce an execution event
                        let execution_result = process_state_transition(
                            &platform_ref,
                            block_info,
                            state_transition,
                            Some(transaction),
                        )
                        .map(|validation_result| {
                            // Dispatch to the versioned helper (v0 = pre-v13, v1 = records
                            // paid-invalid balance effects).
                            self.process_validation_result(
                                raw_state_transition,
                                &state_transition_name,
                                validation_result,
                                block_info,
                                transaction,
                                platform_version,
                                platform_ref.state.previous_fee_versions(),
                            )
                            .unwrap_or_else(error_to_internal_error_execution_result)
                        })
                        .map_err(|error| StateTransitionAwareError {
                            error,
                            raw_state_transition,
                            state_transition_name: Some(state_transition_name.to_string()),
                        })
                        .unwrap_or_else(error_to_internal_error_execution_result);

                        #[cfg(test)]
                        let execution_result = if matches!(
                            execution_result,
                            StateTransitionExecutionResult::SuccessfulExecution { .. }
                        )
                            && test_fault_injection::FAIL_NEXT_SUCCESSFUL_EXECUTION
                                .with(|flag| flag.replace(false))
                        {
                            StateTransitionExecutionResult::InternalError(
                                "injected post-apply failure (test_fault_injection)".to_string(),
                            )
                        } else {
                            execution_result
                        };

                        match &execution_result {
                            StateTransitionExecutionResult::InternalError(_)
                            | StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                                // This transition will be stripped from the block
                                // (`TxAction::Removed`), so none of its writes may remain in
                                // the state the app hash is computed over. A rollback failure
                                // means we can no longer produce a state matching the block —
                                // fail the whole proposal rather than continue on leaked state.
                                transaction.rollback_to_savepoint().map_err(|e| {
                                    drive::grovedb::error::Error::StorageError(RocksDBError(e))
                                })?;
                            }
                            _ => {
                                // The transition stays in the block, so its writes stay. Its
                                // savepoint is intentionally left on the stack (see the method
                                // documentation).
                            }
                        }

                        // Store metrics
                        let elapsed_time = start_time.elapsed() + decoding_elapsed_time;

                        let code = match &execution_result {
                            StateTransitionExecutionResult::SuccessfulExecution { .. } => 0,
                            StateTransitionExecutionResult::PaidConsensusError {
                                error, ..
                            } => error.code(),
                            StateTransitionExecutionResult::UnpaidConsensusError(error) => {
                                error.code()
                            }
                            StateTransitionExecutionResult::InternalError(_) => 1,
                            StateTransitionExecutionResult::NotExecuted(_) => 1, //todo
                        };

                        state_transition_execution_histogram(
                            elapsed_time,
                            &state_transition_name,
                            code,
                        );

                        execution_result
                    }
                    DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                        raw,
                        error,
                        elapsed_time: decoding_elapsed_time,
                    }) => {
                        if tracing::enabled!(tracing::Level::DEBUG) {
                            let st_hash = hex::encode(hash_single(raw));

                            tracing::debug!(
                                ?error,
                                st_hash,
                                "Invalid unknown state transition ({}): {}",
                                st_hash,
                                error
                            );
                        }

                        // Store metrics
                        state_transition_execution_histogram(
                            decoding_elapsed_time,
                            "Unknown",
                            error.code(),
                        );

                        StateTransitionExecutionResult::UnpaidConsensusError(error)
                    }
                    DecodedStateTransition::FailedToDecode(
                        InvalidWithProtocolErrorStateTransition {
                            raw,
                            error: protocol_error,
                            elapsed_time: decoding_elapsed_time,
                        },
                    ) => {
                        // Store metrics
                        state_transition_execution_histogram(decoding_elapsed_time, "Unknown", 1);

                        error_to_internal_error_execution_result(StateTransitionAwareError {
                            error: protocol_error.into(),
                            raw_state_transition: raw,
                            state_transition_name: None,
                        })
                    }
                }
            };

            processing_result.add(execution_result)?;
        }

        Ok(processing_result)
    }
}

fn error_to_internal_error_execution_result(
    error_with_st: StateTransitionAwareError,
) -> StateTransitionExecutionResult {
    if tracing::enabled!(tracing::Level::ERROR) {
        let st_hash = hex::encode(hash_single(error_with_st.raw_state_transition));

        tracing::error!(
            error = ?error_with_st.error,
            raw_state_transition = ?error_with_st.raw_state_transition,
            st_hash,
            "Failed to process {} state transition ({}) : {}",
            error_with_st.state_transition_name.unwrap_or_else(|| "unknown".to_string()),
            st_hash,
            error_with_st.error,
        );
    }

    StateTransitionExecutionResult::InternalError(error_with_st.error.to_string())
}
