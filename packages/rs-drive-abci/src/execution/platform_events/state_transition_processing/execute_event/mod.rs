mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Executes the given `event` based on the `block_info` and `transaction`.
    ///
    /// # Arguments
    ///
    /// * `event` - The execution event to be processed.
    /// * `block_info` - Information about the current block being processed.
    /// * `transaction` - The transaction associated with the execution event.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<ExecutionResult, Error>` - If the execution is successful, it returns an `ExecutionResult`
    ///   which can be one of the following variants: `SuccessfulPaidExecution`, `SuccessfulFreeExecution`, or
    ///   `ConsensusExecutionError`. If the execution fails, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with the drive operations or
    /// an internal error occurs.
    pub(in crate::execution) fn execute_event(
        &self,
        event: ExecutionEvent,
        consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<EventExecutionResult, Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .execute_event
        {
            0 => self.execute_event_v0(
                event,
                consensus_errors,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "execute_event".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
