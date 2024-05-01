use crate::error::Error;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore;
use dpp::dashcore::hashes::Hash;
use dpp::fee::fee_result::FeeResult;
use dpp::validation::ConsensusValidationResult;

use crate::execution::types::execution_event::ExecutionEvent;
use crate::execution::types::state_transition_container::v0::StateTransitionContainerGettersV0;
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::state_transitions_processing_result::{
    StateTransitionExecutionResult, StateTransitionsProcessingResult,
};
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

struct StateTransitionAwareError {
    error: Error,
    raw_state_transition: Vec<u8>,
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
        raw_state_transitions: &Vec<Vec<u8>>,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransitionsProcessingResult, Error> {
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: block_platform_state,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };

        let state_transition_container =
            self.decode_raw_state_transitions(raw_state_transitions, platform_version)?;

        let (
            valid_state_transitions,
            invalid_state_transitions,
            invalid_execution_abci_state_transitions,
        ) = state_transition_container.destructure();

        let mut processing_result = StateTransitionsProcessingResult::default();

        for (raw_state_transition, state_transition) in valid_state_transitions {
            tracing::trace!(?state_transition, "Processing state transition");

            let state_transition_name = state_transition.name();

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
                    state_transition_name,
                    validation_result,
                    block_info,
                    transaction,
                    platform_version,
                )
                .unwrap_or_else(|execution_error| {
                    let mut st_hash = String::new();
                    if tracing::enabled!(tracing::Level::ERROR) {
                        st_hash = hex::encode(
                            dashcore::hashes::sha256::Hash::hash(raw_state_transition)
                                .to_byte_array(),
                        );
                    }

                    tracing::error!(
                        error = ?execution_error.error,
                        raw_state_transition = ?execution_error.raw_state_transition,
                        "Internal Error processing state transition ({}) : {}",
                        st_hash,
                        execution_error.error,
                    );

                    StateTransitionExecutionResult::InternalError(execution_error.error.to_string())
                })
            })
            .unwrap_or_else(|processing_error| {
                let mut st_hash = String::new();
                if tracing::enabled!(tracing::Level::ERROR) {
                    st_hash = hex::encode(
                        dashcore::hashes::sha256::Hash::hash(raw_state_transition).to_byte_array(),
                    );
                }

                tracing::error!(
                    error = ?processing_error,
                    raw_state_transition = ?raw_state_transition,
                    "Internal Error processing state transition ({}) : {}",
                    st_hash,
                    processing_error,
                );

                StateTransitionExecutionResult::InternalError(processing_error.to_string())
            });

            processing_result.add(execution_result)?;
        }

        for (_, consensus_error) in invalid_state_transitions {
            // we have already traced error messages, no need to create new ones
            processing_result.add(StateTransitionExecutionResult::UnpaidConsensusError(
                consensus_error,
            ))?;
        }

        for (_, protocol_error) in invalid_execution_abci_state_transitions {
            // we have already traced error messages, no need to create new ones
            processing_result.add(StateTransitionExecutionResult::InternalError(format!(
                "{}",
                protocol_error
            )))?;
        }

        Ok(processing_result)
    }

    fn process_validation_result_v0(
        &self,
        raw_state_transition: &[u8], //used for errors
        state_transition_name: &str,
        mut validation_result: ConsensusValidationResult<ExecutionEvent>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransitionExecutionResult, StateTransitionAwareError> {
        // Tenderdash hex-encoded ST hash
        let mut st_hash = String::new();

        // State Transition is invalid
        if !validation_result.is_valid() {
            let first_consensus_error = validation_result
                .errors
                // the first error must be present for an invalid result
                .remove(0);

            if tracing::enabled!(tracing::Level::DEBUG) {
                st_hash = hex::encode(
                    dashcore::hashes::sha256::Hash::hash(raw_state_transition).to_byte_array(),
                );
            }

            tracing::debug!(
                errors = ?validation_result.errors,
                "Invalid {} state transition ({}): {}",
                state_transition_name,
                st_hash,
                &first_consensus_error
            );

            // To prevent spam we should deduct fees for invalid state transitions as well.
            // There are two cases when the user can't pay fees:
            // 1. The state transition is funded by an asset lock transactions. This transactions are
            //    placed on the payment blockchain and they can't be partially spent.
            // 2. We can't prove that the state transition is associated with the identity
            // 3. The revision given by the state transition isn't allowed based on the state
            let state_transition_execution_result = if let Ok((execution_event, errors)) =
                validation_result.into_data_and_errors()
            {
                // In this case the execution event will be to pay for the state transition processing
                // This ONLY pays for what is needed to prevent attacks on the system

                let event_execution_result = self
                    .execute_event(
                        execution_event,
                        errors,
                        block_info,
                        transaction,
                        platform_version,
                    )
                    .map_err(|error| StateTransitionAwareError {
                        error,
                        raw_state_transition: raw_state_transition.to_vec(),
                    })?;

                match event_execution_result {
                    EventExecutionResult::SuccessfulPaidExecution(_, actual_fees)
                    | EventExecutionResult::UnsuccessfulPaidExecution(_, actual_fees, _) => {
                        tracing::debug!(
                            "{} state transition ({}) not processed, but paid for processing",
                            state_transition_name,
                            st_hash,
                        );

                        StateTransitionExecutionResult::PaidConsensusError(
                            first_consensus_error,
                            actual_fees,
                        )
                    }
                    EventExecutionResult::SuccessfulFreeExecution => {
                        tracing::debug!(
                            "Free {} state transition ({}) successfully processed",
                            state_transition_name,
                            st_hash,
                        );

                        StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
                    }
                    EventExecutionResult::UnpaidConsensusExecutionError(mut errors) => {
                        let payment_consensus_error = errors
                            // the first error must be present for an invalid result
                            .remove(0);

                        tracing::debug!(
                            main_error = ?first_consensus_error,
                            payment_error = ?payment_consensus_error,
                            "Not able to reduce balance for identity {} state transition ({})",
                            state_transition_name,
                            st_hash,
                        );

                        StateTransitionExecutionResult::InternalError(format!(
                            "{} {}",
                            first_consensus_error, payment_consensus_error
                        ))
                    }
                }
            } else {
                StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
            };

            return Ok(state_transition_execution_result);
        }

        let (execution_event, errors) =
            validation_result.into_data_and_errors().map_err(|error| {
                StateTransitionAwareError {
                    error: error.into(),
                    raw_state_transition: raw_state_transition.to_vec(),
                }
            })?;

        let event_execution_result = self
            .execute_event(
                execution_event,
                errors,
                block_info,
                transaction,
                platform_version,
            )
            .map_err(|error| StateTransitionAwareError {
                error,
                raw_state_transition: raw_state_transition.to_vec(),
            })?;

        let state_transition_execution_result = match event_execution_result {
            EventExecutionResult::SuccessfulPaidExecution(estimated_fees, actual_fees) => {
                tracing::debug!(
                    "{} state transition ({}) successfully processed",
                    state_transition_name,
                    st_hash,
                );

                StateTransitionExecutionResult::SuccessfulExecution(estimated_fees, actual_fees)
            }
            EventExecutionResult::UnsuccessfulPaidExecution(_, actual_fees, mut errors) => {
                tracing::debug!(
                    "{} state transition ({}) not successfully processed",
                    state_transition_name,
                    st_hash,
                );

                let payment_consensus_error = errors
                    // the first error must be present for an invalid result
                    .remove(0);

                StateTransitionExecutionResult::PaidConsensusError(
                    payment_consensus_error,
                    actual_fees,
                )
            }
            EventExecutionResult::SuccessfulFreeExecution => {
                tracing::debug!(
                    "Free {} state transition ({}) successfully processed",
                    state_transition_name,
                    st_hash,
                );

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

                tracing::debug!(
                    error = ?first_consensus_error,
                    "Insufficient identity balance to process {} state transition ({})",
                    state_transition_name,
                    st_hash,
                );

                StateTransitionExecutionResult::UnpaidConsensusError(first_consensus_error)
            }
        };

        Ok(state_transition_execution_result)
    }
}
