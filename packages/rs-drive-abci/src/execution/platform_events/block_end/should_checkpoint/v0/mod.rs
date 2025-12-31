use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::drive::CheckpointInfo;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Information needed to create a checkpoint
pub struct CheckpointNeededInfo {
    /// The block height for the checkpoint
    pub block_height: u64,
    /// The block time for the checkpoint
    pub block_time: u64,
    /// The current checkpoints (already loaded, to avoid reloading)
    pub current_checkpoints: Arc<BTreeMap<u64, CheckpointInfo>>,
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determines whether a checkpoint should be created for the current block.
    ///
    /// Returns `Ok(Some(CheckpointNeededInfo))` if a checkpoint should be created,
    /// `Ok(None)` if no checkpoint is needed.
    #[inline(always)]
    pub(super) fn should_checkpoint_v0(
        &self,
        block_execution_context: &BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Option<CheckpointNeededInfo>, Error> {
        // Check if checkpoints are disabled in testing config
        #[cfg(feature = "testing-config")]
        if self.config.testing_configs.disable_checkpoints {
            return Ok(None);
        }

        // How often we want a checkpoint
        let checkpoint_interval_milliseconds =
            platform_version.drive_abci.checkpoints.frequency_seconds as u64 * 1000;
        let keep_n = platform_version.drive_abci.checkpoints.num_checkpoints as usize;

        // If disabled or misconfigured, do nothing.
        if checkpoint_interval_milliseconds == 0 || keep_n == 0 {
            return Ok(None);
        }

        let block_info = block_execution_context.block_state_info();
        let block_time = block_info.block_time_ms();
        let block_height = block_info.height();

        let most_recent_checkpoint_interval_time =
            block_time - block_time % checkpoint_interval_milliseconds;

        // Load current checkpoints
        let current_checkpoints_guard = self.drive.checkpoints.load();

        // Determine whether we should checkpoint based on the last checkpoint timestamp
        let should_checkpoint = match current_checkpoints_guard.last_key_value() {
            None => true,
            Some((_height, checkpoint_info)) => {
                checkpoint_info.timestamp_ms < most_recent_checkpoint_interval_time
                    && block_time >= most_recent_checkpoint_interval_time
            }
        };

        if should_checkpoint {
            // Clone the Arc to take ownership
            let current_checkpoints = Arc::clone(&current_checkpoints_guard);
            Ok(Some(CheckpointNeededInfo {
                block_height,
                block_time,
                current_checkpoints,
            }))
        } else {
            Ok(None)
        }
    }
}
