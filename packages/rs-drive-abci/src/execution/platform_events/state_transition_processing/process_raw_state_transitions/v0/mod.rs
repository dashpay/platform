use crate::error::Error;
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult::{
    ConsensusExecutionError, SuccessfulPaidExecution,
};
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;

use crate::platform_types::state_transition_execution_result::StateTransitionExecutionResult;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

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
    /// * `Result<(FeeResult, Vec<ExecTxResult>), Error>` - If the processing is successful, it returns
    ///   a tuple consisting of a `FeeResult` and a vector of `ExecTxResult`. If the processing fails,
    ///   it returns an `Error`.
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
    ) -> Result<(FeeResult, Vec<(Vec<u8>, StateTransitionExecutionResult)>), Error> {
        let state_transitions = StateTransition::deserialize_many(raw_state_transitions)?;
        let mut aggregate_fee_result = FeeResult::default();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: block_platform_state,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };
        let exec_tx_results = state_transitions
            .into_iter()
            .zip(raw_state_transitions.iter())
            .map(|(state_transition, raw_state_transition)| {
                let state_transition_execution_event = process_state_transition(
                    &platform_ref,
                    state_transition.clone(),
                    Some(transaction),
                )?;

                let execution_result = if state_transition_execution_event.is_valid() {
                    let execution_event = state_transition_execution_event.into_data()?;

                    let result = self.execute_event(
                        execution_event,
                        block_info,
                        transaction,
                        platform_version,
                    )?;

                    if tracing::enabled!(tracing::Level::TRACE) {
                        tracing::trace!(
                            method = "process_raw_state_transitions_v0",
                            ?state_transition,
                            block_platform_state_fingerprint =
                                hex::encode(block_platform_state.fingerprint()),
                            "State transition successfully processed",
                        );
                    }

                    result
                } else {
                    // Re-enable this to see errors during testing
                    // dbg!(
                    //     state_transition,
                    //     state_transition_execution_event.errors.first().clone()
                    // );

                    if tracing::enabled!(tracing::Level::TRACE) {
                        tracing::trace!(
                            method = "process_raw_state_transitions_v0",
                            ?state_transition,
                            block_platform_state_fingerprint =
                                hex::encode(block_platform_state.fingerprint()),
                            "Invalid state transition: {:?}",
                            state_transition_execution_event.errors.first().clone(),
                        );
                    }

                    ConsensusExecutionError(SimpleConsensusValidationResult::new_with_errors(
                        state_transition_execution_event.errors,
                    ))
                };
                if let SuccessfulPaidExecution(_, fee_result) = &execution_result {
                    aggregate_fee_result.checked_add_assign(fee_result.clone())?;
                }

                Ok((raw_state_transition.clone(), execution_result))
            })
            .collect::<Result<Vec<(Vec<u8>, StateTransitionExecutionResult)>, Error>>()?;

        Ok((aggregate_fee_result, exec_tx_results))
    }
}
