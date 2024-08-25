use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pool withdrawal documents into transactions. This function is a version handler that
    /// directs to specific version implementations of the `pool_withdrawals_into_transactions_queue` function.
    ///
    /// # Arguments
    ///
    /// * `block_execution_context` - A `BlockExecutionContext` reference that provides context for block execution.
    /// * `transaction` - A `Transaction` reference representing the current transaction.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns `Ok(())` if successful, otherwise returns an `Error`.
    pub(in crate::execution) fn pool_withdrawals_into_transactions_queue(
        &self,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .pool_withdrawals_into_transactions_queue
        {
            0 => self.pool_withdrawals_into_transactions_queue_v0(
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "pool_withdrawals_into_transactions_queue".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
