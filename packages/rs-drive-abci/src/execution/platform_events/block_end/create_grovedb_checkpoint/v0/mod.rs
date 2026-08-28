use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::serialization::PlatformSerializable;
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
    /// Creates a GroveDB checkpoint for the currently committed state.
    ///
    /// This should be called AFTER the transaction is committed, so the checkpoint
    /// captures the committed state. It also saves the platform state to the
    /// checkpoint directory and updates the checkpoint_platform_states cache.
    ///
    /// # Arguments
    ///
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Ok if the checkpoint was created successfully.
    ///
    #[inline(always)]
    pub(super) fn create_grovedb_checkpoint_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let platform_state = self.state.load();
        let block_height = platform_state.last_committed_block_height();
        let block_time = platform_state.last_committed_block_time_ms().unwrap_or(0);

        // When snapshot serving is enabled, the operator-provided state sync
        // configuration overrides the platform-version-driven checkpoint retention.
        let state_sync_config = &self.config.abci.state_sync;
        let keep_n = if state_sync_config.snapshots_enabled {
            state_sync_config.max_num_snapshots
        } else {
            platform_version.drive_abci.checkpoints.num_checkpoints as usize
        };

        // Build the checkpoint path: <checkpoints_path>/<block_height>
        // (defaults to db_path/checkpoints)
        let checkpoint_path = state_sync_config
            .resolved_checkpoints_path(&self.config.db_path)
            .join(block_height.to_string());

        // Create the parent checkpoints directory if it doesn't exist
        std::fs::create_dir_all(checkpoint_path.parent().unwrap()).map_err(|err| {
            Error::Drive(IOErrorWithInfoString(
                err.into(),
                "trying to create checkpoints directory".to_owned(),
            ))
        })?;

        // Create checkpoint DB (this creates the checkpoint_path directory)
        self.drive.grove.create_checkpoint(&checkpoint_path)?;

        // Save platform state to checkpoint directory on disk (after grovedb creates the directory)
        let checkpoint_state_path = checkpoint_path.join("platform_state.bin");
        let state_bytes = platform_state.serialize_to_bytes()?;
        std::fs::write(&checkpoint_state_path, &state_bytes).map_err(|err| {
            Error::Drive(IOErrorWithInfoString(
                err.into(),
                format!(
                    "trying to write checkpoint platform state to {:?}",
                    checkpoint_state_path
                ),
            ))
        })?;

        // Open the checkpoint as a GroveDb instance and wrap it in a Checkpoint
        let checkpoint_db = GroveDb::open(&checkpoint_path)?;
        let checkpoint = Checkpoint::new(checkpoint_db, checkpoint_path);

        // Load current checkpoints
        let current_checkpoints = self.drive.checkpoints.load();

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

        // Update checkpoint_platform_states cache
        let current_checkpoint_states = self.checkpoint_platform_states.load();
        let mut new_checkpoint_states = BTreeMap::new();

        // Add the current platform state for this new checkpoint
        new_checkpoint_states.insert(block_height, Arc::clone(&platform_state));

        // Copy only the states for checkpoints we're keeping
        for (height, state) in current_checkpoint_states.iter().skip(to_skip) {
            new_checkpoint_states.insert(*height, Arc::clone(state));
        }

        // Atomically swap in the new checkpoint states map
        self.checkpoint_platform_states
            .store(Arc::new(new_checkpoint_states));

        Ok(())
    }
}
