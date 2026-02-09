mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Creates a GroveDB checkpoint for the currently committed state.
    ///
    /// This should be called AFTER the transaction is committed, so the checkpoint
    /// captures the committed state.
    ///
    /// # Arguments
    ///
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if the checkpoint was created successfully.
    ///
    pub fn create_grovedb_checkpoint(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .update_checkpoints
        {
            None => Ok(()), // Checkpoints disabled for this version
            Some(0) => self.create_grovedb_checkpoint_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_grovedb_checkpoint".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
