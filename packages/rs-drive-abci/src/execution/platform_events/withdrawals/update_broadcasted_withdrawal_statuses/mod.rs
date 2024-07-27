use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update statuses for broadcasted withdrawals
    ///
    /// This method is responsible for updating the status of withdrawal transactions that have been broadcasted and reached finality.
    /// This is done based on the height of the last synced core block, which helps in determining whether the withdrawal
    /// transaction has been completed or expired.
    ///
    /// # Arguments
    ///
    /// * `block_execution_context` - Contextual information about the current block execution.
    /// * `transaction` - A transaction argument to interact with the underlying storage.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns an Ok(()) if the statuses are successfully updated.
    ///                          Otherwise, it returns an Error.
    ///
    /// # Errors
    ///
    /// This function may return an error if any of the following conditions are met:
    ///
    /// * There is an issue interacting with the underlying storage.
    /// * There is an error fetching the withdrawal data contract.
    /// * There is an error getting the transactionId or transactionSignHeight from the withdrawal document.
    pub fn update_broadcasted_withdrawal_statuses(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .update_broadcasted_withdrawal_statuses
        {
            0 => self.update_broadcasted_withdrawal_statuses_v0(
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_broadcasted_withdrawal_statuses".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
