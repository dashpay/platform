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
    /// Cleans up expired locks of withdrawal amounts based on the protocol version.
    ///
    /// This function determines the appropriate versioned function to call for cleaning up expired
    /// withdrawal locks according to the provided platform version. It deletes expired withdrawal locks
    /// that have surpassed their allowed time limit, ensuring that only valid withdrawal entries remain.
    ///
    /// # Parameters
    /// * `block_info`: Information about the current block, including the timestamp used to identify expired locks.
    /// * `transaction`: The transaction under which this operation should be performed.
    /// * `platform_version`: The version of the platform to ensure compatibility with the appropriate cleanup method.
    ///
    /// # Returns
    /// * `Ok(())`: If the cleanup was performed successfully.
    /// * `Err(Error::Execution(ExecutionError::UnknownVersionMismatch))`: If the platform version does not match known versions.
    ///
    /// # Errors
    /// Returns an error if the platform version is unknown or if the cleanup process encounters an issue.
    pub fn clean_up_expired_locks_of_withdrawal_amounts(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .withdrawals
            .cleanup_expired_locks_of_withdrawal_amounts
        {
            0 => self.cleanup_expired_locks_of_withdrawal_amounts_v0(
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "cleanup_expired_locks_of_withdrawal_amounts".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
