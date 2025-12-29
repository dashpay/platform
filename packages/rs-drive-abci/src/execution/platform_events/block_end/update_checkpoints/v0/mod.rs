use crate::error::Error;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use drive::drive::{Checkpoint, CheckpointInfo};
use drive::error::Error::IOErrorWithInfoString;
use drive::grovedb::GroveDb;
use std::collections::BTreeMap;
use std::sync::Arc;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates checkpoints
    ///
    /// Returns `true` if a new checkpoint was created, `false` otherwise.
    #[inline(always)]
    pub(super) fn update_checkpoints_v0(
        &self,
        block_execution_context: &BlockExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        // Check if checkpoints are disabled in testing config
        #[cfg(feature = "testing-config")]
        if self.config.testing_configs.disable_checkpoints {
            return Ok(false);
        }

        // How often we want a checkpoint
        let checkpoint_interval_milliseconds =
            platform_version.drive_abci.checkpoints.frequency_seconds as u64 * 1000;
        let keep_n = platform_version.drive_abci.checkpoints.num_checkpoints as usize;

        // If disabled or misconfigured, do nothing.
        if checkpoint_interval_milliseconds == 0 || keep_n == 0 {
            return Ok(false);
        }

        let block_info = block_execution_context.block_state_info();
        let block_time = block_info.block_time_ms();
        let block_height = block_info.height();

        let most_recent_checkpoint_interval_time =
            block_time - block_time % checkpoint_interval_milliseconds;

        // Load current checkpoints
        let current_checkpoints = self.drive.checkpoints.load();

        // Determine whether we should checkpoint based on the last checkpoint timestamp
        let should_checkpoint = match current_checkpoints.last_key_value() {
            None => true,
            Some((_height, checkpoint_info)) => {
                checkpoint_info.timestamp_ms < most_recent_checkpoint_interval_time
                    && block_time >= most_recent_checkpoint_interval_time
            }
        };

        if !should_checkpoint {
            return Ok(false);
        }

        // Build the checkpoint path: db_path/checkpoints/<block_height>
        let checkpoint_path = self
            .config
            .db_path
            .join("checkpoints")
            .join(block_height.to_string());

        // Create the checkpoints directory if it doesn't exist
        std::fs::create_dir_all(checkpoint_path.parent().unwrap()).map_err(|err| {
            Error::Drive(IOErrorWithInfoString(
                err.into(),
                "trying to create checkpoint directory".to_owned(),
            ))
        })?;

        // Create checkpoint DB
        self.drive.grove.create_checkpoint(&checkpoint_path)?;

        // Open the checkpoint as a GroveDb instance and wrap it in a Checkpoint
        let checkpoint_db = GroveDb::open(&checkpoint_path)?;
        let checkpoint = Checkpoint::new(checkpoint_db, checkpoint_path);

        // Calculate how many old checkpoints we can keep (reserving 1 slot for the new one)
        let max_old_to_keep = keep_n.saturating_sub(1);
        let existing_count = current_checkpoints.len();
        let to_skip = existing_count.saturating_sub(max_old_to_keep);

        // Mark old checkpoints that we're not keeping for deletion
        // They will be cleaned up when their Arc reference count drops to zero
        for (_, checkpoint_info) in current_checkpoints.iter().take(to_skip) {
            checkpoint_info.checkpoint.mark_for_deletion();
        }

        // Build new map with only the checkpoints we want to keep
        let mut new_checkpoints = BTreeMap::new();

        // Add the new checkpoint
        new_checkpoints.insert(
            block_height,
            CheckpointInfo::new(block_time, Arc::new(checkpoint)),
        );

        // Copy only the most recent old checkpoints (skip the oldest ones)
        // BTreeMap iterates in ascending order, so skip the first `to_skip` entries
        for (height, value) in current_checkpoints.iter().skip(to_skip) {
            new_checkpoints.insert(*height, value.clone());
        }

        // Atomically swap in the new checkpoints map
        self.drive.checkpoints.store(Arc::new(new_checkpoints));

        Ok(true)
    }
}
