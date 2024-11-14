mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Validates the fees of a given `ExecutionEvent`.
    ///
    /// # Arguments
    ///
    /// * `event` - The `ExecutionEvent` instance to validate.
    /// * `block_info` - Information about the current block.
    /// * `transaction` - The transaction arguments for the given event.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<ConsensusValidationResult<FeeResult>, Error>` - On success, returns a
    ///   `ConsensusValidationResult` containing an `FeeResult`. On error, returns an `Error`.
    ///
    /// # Errors
    ///
    /// * This function may return an `Error::Execution` if the identity balance is not found.
    /// * This function may return an `Error::Drive` if there's an issue with applying drive operations.
    pub(in crate::execution) fn validate_fees_of_event(
        &self,
        event: &ExecutionEvent,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: &CachedEpochIndexFeeVersions,
    ) -> Result<ConsensusValidationResult<FeeResult>, Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .validate_fees_of_event
        {
            0 => self.validate_fees_of_event_v0(
                event,
                block_info,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "validate_fees_of_event".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
