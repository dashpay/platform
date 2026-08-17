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
/// `apply_drive_operations(apply = true)`, e.g. the address-input fee coverage guard failing
/// on an under-estimated `Shield`) without depending on any particular estimation bug.
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
    /// This function takes a reference to a vector of raw state transitions, `BlockInfo`, and a `Transaction`
    /// as input and performs the corresponding state transition operations. It deserializes the raw state
    /// transitions into a `StateTransition` and processes them.
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
    /// state transitions, processing state transitions, or executing events.
    ///
    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_raw_state_transitions_v0(
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

                        // PROPOSER-SIDE ONLY (consensus-invisible, hence no protocol-version
                        // gate): when building a proposal, mark the state before this
                        // transition. Execution can write into the shared block transaction
                        // before failing (the address-input fee flow is apply-then-check), and
                        // a transition whose result strips it from the block
                        // (`TxAction::Removed`) must leave no trace in the state the proposal's
                        // app hash is computed over — otherwise the gossiped block omits the
                        // transition while the advertised app hash includes its writes, no
                        // validator can reproduce the hash, and every proposer carrying the
                        // transition burns its round (mainnet evo1 stalls of 2026-08-14/15,
                        // after heights 415652 and 415661).
                        //
                        // The validation path (`proposing_state_transitions == false`) is
                        // deliberately untouched: rolling back there would change what state a
                        // received block evaluates to, which is a consensus change that must
                        // ride a protocol-version gate (it does, from v14). This proposer-side
                        // rollback only changes which blocks this node BUILDS — the published
                        // block and app hash are exactly what any un-upgraded validator
                        // computes from that block, so mixed networks cannot diverge.
                        if proposing_state_transitions {
                            transaction.set_savepoint();
                        }

                        // Validate state transition and produce an execution event
                        let execution_result = process_state_transition(
                            &platform_ref,
                            block_info,
                            state_transition,
                            Some(transaction),
                        )
                        .map(|validation_result| {
                            // Dispatch to the versioned helper (v0 = pre-v13, v1 = records
                            // paid-invalid balance effects). Only this helper changed at v13; the outer
                            // loop is version-agnostic, so it must NOT be versioned.
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

                        if proposing_state_transitions {
                            match &execution_result {
                                StateTransitionExecutionResult::InternalError(_)
                                | StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                                    // This transition will be stripped from the proposal
                                    // (`TxAction::Removed`), so none of its writes may remain
                                    // in the state the app hash is computed over. A rollback
                                    // failure means the proposal can no longer match the
                                    // block — fail it rather than continue on leaked state.
                                    transaction.rollback_to_savepoint().map_err(|e| {
                                        drive::grovedb::error::Error::StorageError(RocksDBError(e))
                                    })?;
                                }
                                _ => {
                                    // The transition stays in the block, so its writes stay.
                                    // Its savepoint is intentionally left on the stack:
                                    // RocksDB exposes no pop-without-rollback, leftover
                                    // savepoints are inert for commit, and the per-round
                                    // proposal transaction they live in is dropped when the
                                    // round ends. The genesis re-proposal path — the one other
                                    // consumer of this stack — drains the whole stack rather
                                    // than popping once, so this residue cannot redirect it.
                                }
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
