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
    /// Clears the drive block cache at the start of processing a block. This does a few things like clearing
    /// the block data contract cache and the block platform versions cache.
    ///
    /// This function is a version handler that directs to specific version implementations
    /// of the update_state_cache function.
    ///
    /// # Arguments
    ///
    /// * `platform_version` - A `PlatformVersion` reference that dictates which version of
    ///   the method to call.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - If the state cache and quorums are successfully updated, it returns `Ok(())`.
    ///   If there is a problem with the update, it returns an `Error`.
    ///
    pub fn clear_drive_block_cache(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_start
            .clear_drive_block_cache
        {
            0 => {
                self.clear_drive_block_cache_v0();
                Ok(())
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clear_drive_block_cache".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
