use crate::error::Error;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::fee_result::FeeResult;

use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, InvalidStateTransition, InvalidWithProtocolErrorStateTransition,
    SuccessfullyDecodedStateTransition,
};
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::metrics::{state_transition_execution_histogram, HistogramTiming};
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::{
    NotExecutedReason, StateTransitionExecutionResult, StateTransitionsProcessingResult,
};
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::prelude::TimestampMillis;
use dpp::util::hash::hash_single;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::time::Instant;

#[derive(Debug)]
struct StateTransitionAwareError<'t> {
    error: Error,
    raw_state_transition: &'t [u8],
    state_transition_name: Option<String>,
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
            let execution_result = if proposing_state_transitions
                && timer.map_or(false, |timer| {
                    timer.elapsed().as_millis() as TimestampMillis
                        > self.config.abci.tx_processing_time_limit
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

                        // Validate state transition and produce an execution event
                        let execution_result = process_state_transition(
                            &platform_ref,
                            block_info,
                            state_transition,
                            Some(transaction),
                        )
                        .map(|validation_result| {
                            self.process_validation_result_v0(
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

                        // Store metrics
                        let elapsed_time = start_time.elapsed() + decoding_elapsed_time;

                        let code = match &execution_result {
                            StateTransitionExecutionResult::SuccessfulExecution(_, _) => 0,
                            StateTransitionExecutionResult::PaidConsensusError(error, _)
                            | StateTransitionExecutionResult::UnpaidConsensusError(error) => {
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

    fn process_validation_result_v0<'a>(
        &self,
        raw_state_transition: &'a [u8], //used for errors
        state_transition_name: &str,
        mut validation_result: ConsensusValidationResult<ExecutionEvent>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<StateTransitionExecutionResult, StateTransitionAwareError<'a>> {
        // State Transition is invalid
        if !validation_result.is_valid() {
            // To prevent spam we should deduct fees for invalid state transitions as well.
            // There are three cases when the user can't pay fees:
            // 1. The state transition is funded by an asset lock transactions. This transactions are
            //    placed on the payment blockchain and they can't be partially spent.
            // 2. We can't prove that the state transition is associated with the identity
            // 3. The revision given by the state transition isn't allowed based on the state
            if validation_result.data.is_none() {
                let first_consensus_error = validation_result
                    .errors
                    // the first error must be present for an invalid result
                    .remove(0);

                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        error = ?first_consensus_error,
                        st_hash,
                        "Invalid {} state transition without identity ({}): {}",
                        state_transition_name,
                        st_hash,
                        &first_consensus_error
                    );
                }

                // We don't have execution event, so we can't pay for processing
                return Ok(StateTransitionExecutionResult::UnpaidConsensusError(
                    first_consensus_error,
                ));
            };

            let (execution_event, errors) = validation_result
                .into_data_and_errors()
                .expect("data must be present since we check it few lines above");

            let first_consensus_error = errors
                .first()
                .expect("error must be present since we check it few lines above")
                .clone();

            // In this case the execution event will be to pay for the state transition processing
            // This ONLY pays for what is needed to prevent attacks on the system

            let event_execution_result = self
                .execute_event(
                    execution_event,
                    errors,
                    block_info,
                    transaction,
                    platform_version,
                    previous_fee_versions,
                )
                .map_err(|error| StateTransitionAwareError {
                    error,
                    raw_state_transition,
                    state_transition_name: Some(state_transition_name.to_string()),
                })?;

            let state_transition_execution_result = match event_execution_result {
                EventExecutionResult::SuccessfulPaidExecution(estimated_fees, actual_fees)
                | EventExecutionResult::UnsuccessfulPaidExecution(estimated_fees, actual_fees, _) =>
                {
                    if tracing::enabled!(tracing::Level::DEBUG) {
                        let st_hash = hex::encode(hash_single(raw_state_transition));

                        tracing::debug!(
                            error = ?first_consensus_error,
                            st_hash,
                            ?estimated_fees,
                            ?actual_fees,
                            "Invalid {} state transition ({}): {}",
                            state_transition_name,
                            st_hash,
                            &first_consensus_error
                        );
                    }

                    StateTransitionExecutionResult::PaidConsensusError(
                        first_consensus_error,
                        actual_fees,
                    )
                }
                EventExecutionResult::SuccessfulFreeExecution => {
                    if tracing::enabled!(tracing::Level::DEBUG) {
                        let st_hash = hex::encode(hash_single(raw_state_transition));

                        tracing::debug!(
                            error = ?first_consensus_error,
                            st_hash,
                            "Free invalid {} state transition ({}): {}",
                            state_transition_name,
                            st_hash,
                            &first_consensus_error
                        );
                    }

                    StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
                }
                EventExecutionResult::UnpaidConsensusExecutionError(mut payment_errors) => {
                    let payment_consensus_error = payment_errors
                        // the first error must be present for an invalid result
                        .remove(0);

                    if tracing::enabled!(tracing::Level::ERROR) {
                        let st_hash = hex::encode(hash_single(raw_state_transition));

                        tracing::error!(
                            main_error = ?first_consensus_error,
                            payment_error = ?payment_consensus_error,
                            st_hash,
                            "Not able to reduce balance for identity {} state transition ({}): {}",
                            state_transition_name,
                            st_hash,
                            payment_consensus_error
                        );
                    }

                    StateTransitionExecutionResult::InternalError(format!(
                        "{first_consensus_error} {payment_consensus_error}",
                    ))
                }
            };

            return Ok(state_transition_execution_result);
        }

        let (execution_event, errors) =
            validation_result.into_data_and_errors().map_err(|error| {
                StateTransitionAwareError {
                    error: error.into(),
                    raw_state_transition,
                    state_transition_name: Some(state_transition_name.to_string()),
                }
            })?;

        let event_execution_result = self
            .execute_event(
                execution_event,
                errors,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            )
            .map_err(|error| StateTransitionAwareError {
                error,
                raw_state_transition,
                state_transition_name: Some(state_transition_name.to_string()),
            })?;

        let state_transition_execution_result = match event_execution_result {
            EventExecutionResult::SuccessfulPaidExecution(estimated_fees, actual_fees) => {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        ?actual_fees,
                        ?estimated_fees,
                        st_hash,
                        "{} state transition ({}) successfully processed",
                        state_transition_name,
                        st_hash,
                    );
                }

                StateTransitionExecutionResult::SuccessfulExecution(estimated_fees, actual_fees)
            }
            EventExecutionResult::UnsuccessfulPaidExecution(
                estimated_fees,
                actual_fees,
                mut errors,
            ) => {
                let payment_consensus_error = errors
                    // the first error must be present for an invalid result
                    .remove(0);

                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        ?actual_fees,
                        ?estimated_fees,
                        st_hash,
                        "{} state transition ({}) processed and mark as invalid: {}",
                        state_transition_name,
                        st_hash,
                        payment_consensus_error
                    );
                }

                StateTransitionExecutionResult::PaidConsensusError(
                    payment_consensus_error,
                    actual_fees,
                )
            }
            EventExecutionResult::SuccessfulFreeExecution => {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        st_hash,
                        "Free {} state transition ({}) successfully processed",
                        state_transition_name,
                        st_hash,
                    );
                }

                StateTransitionExecutionResult::SuccessfulExecution(None, FeeResult::default())
            }
            EventExecutionResult::UnpaidConsensusExecutionError(mut errors) => {
                // TODO: In case of balance is not enough, we need to reduce balance only for processing fees
                //  and return paid consensus error.
                //  Unpaid consensus error should be only if balance not enough even
                //  to cover processing fees
                let first_consensus_error = errors
                    // the first error must be present for an invalid result
                    .remove(0);

                if tracing::enabled!(tracing::Level::DEBUG) {
                    let st_hash = hex::encode(hash_single(raw_state_transition));

                    tracing::debug!(
                        error = ?first_consensus_error,
                        st_hash,
                        "Insufficient identity balance to process {} state transition ({}): {}",
                        state_transition_name,
                        st_hash,
                        first_consensus_error
                    );
                }

                StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
            }
        };

        Ok(state_transition_execution_result)
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
