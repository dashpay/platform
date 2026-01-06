mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Cleans up expired compacted address balance entries from recent block storage.
    ///
    /// This function removes compacted address balance entries whose expiration
    /// time has passed (older than 1 day from when they were compacted).
    ///
    /// # Arguments
    ///
    /// * `block_info` - Information about the current block (provides current time)
    /// * `transaction` - The database transaction
    /// * `platform_version` - The platform version
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Success or an error if the operation fails
    ///
    pub(in crate::execution) fn cleanup_recent_block_storage_address_balances(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .state_transition_processing
            .cleanup_recent_block_storage_address_balances
        {
            None => Ok(()),
            Some(0) => self.cleanup_recent_block_storage_address_balances_v0(
                block_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "cleanup_recent_block_storage_address_balances".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
