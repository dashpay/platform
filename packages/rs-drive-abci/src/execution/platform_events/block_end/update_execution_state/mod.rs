mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the execution state at the end of finalize block. This is done by overriding the current
    /// platform state cache with the block execution state cache.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the update_state_cache function.
    ///
    /// # Arguments
    ///
    /// * `extended_block_info` - Extended block information for the current block.
    /// * `transaction` - The transaction associated with the block.
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If the state cache and quorums are successfully updated, it returns `Ok(())`.
    ///   If there is a problem with the update, it returns an `Error`.
    ///
    pub fn update_execution_state(
        &self,
        extended_block_info: ExtendedBlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .update_execution_state
        {
            0 => self.update_execution_state_v0(extended_block_info, transaction, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_state_cache".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
