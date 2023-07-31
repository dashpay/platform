use crate::error::Error;
use crate::execution::types::execution_result::ExecutionResult::{
    ConsensusExecutionError, SuccessfulPaidExecution,
};
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use tenderdash_abci::proto::abci::ExecTxResult;

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
    ) -> Result<(FeeResult, Vec<(Vec<u8>, ExecTxResult)>), Error> {
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
                let state_transition_execution_event =
                    process_state_transition(&platform_ref, state_transition, Some(transaction))?;

                let execution_result = if state_transition_execution_event.is_valid() {
                    let execution_event = state_transition_execution_event.into_data()?;
                    self.execute_event(execution_event, block_info, transaction, platform_version)?
                } else {
                    ConsensusExecutionError(SimpleConsensusValidationResult::new_with_errors(
                        state_transition_execution_event.errors,
                    ))
                };
                if let SuccessfulPaidExecution(_, fee_result) = &execution_result {
                    aggregate_fee_result.checked_add_assign(fee_result.clone())?;
                }

                Ok((raw_state_transition.clone(), execution_result.into()))
            })
            .collect::<Result<Vec<(Vec<u8>, ExecTxResult)>, Error>>()?;
        Ok((aggregate_fee_result, exec_tx_results))
    }
}
