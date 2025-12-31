mod v0;

pub use v0::CheckpointNeededInfo;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determines whether a checkpoint should be created for the current block.
    ///
    /// Returns `Ok(Some(CheckpointNeededInfo))` if a checkpoint should be created,
    /// `Ok(None)` if no checkpoint is needed.
    pub fn should_checkpoint(
        &self,
        block_execution_context: &BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Option<CheckpointNeededInfo>, Error> {
        match platform_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint
        {
            None => Ok(None),
            Some(0) => self.should_checkpoint_v0(block_execution_context, platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "should_checkpoint".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
