mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::execution::types::block_execution_context::BlockExecutionContext;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the drive cache at the end of finalize block. This does a few things like merging
    /// the data contract cache and the platform versions cache.
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
    pub fn update_drive_cache(
        &self,
        block_execution_context: &BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .update_drive_cache
        {
            0 => {
                self.update_drive_cache_v0(block_execution_context);
                Ok(())
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_drive_cache".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
