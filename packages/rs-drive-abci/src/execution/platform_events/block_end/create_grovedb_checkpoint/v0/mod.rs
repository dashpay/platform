use crate::config::CheckpointStep;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::rpc::core::CoreRPCLike;
use dpp::serialization::PlatformSerializable;
use dpp::version::PlatformVersion;
use drive::drive::{Checkpoint, CheckpointInfo};
use drive::error::Error::IOErrorWithInfoString;
use drive::grovedb::GroveDb;
use std::collections::BTreeMap;
use std::path::Path;
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
    /// Every attempt, successful or not, is recorded (in memory and on disk) as this
    /// interval's checkpoint attempt so that `should_checkpoint` does not ask again
    /// before the next interval boundary. A failed attempt leaves nothing behind so that the
    /// caller's immediate retry starts clean: a checkpoint directory this attempt
    /// created is removed again, while a directory it did not create (RocksDB
    /// refuses an existing target) is left alone. Nothing is registered in the
    /// caches until the checkpoint is complete.
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

        // Record the attempt before anything that can fail: a checkpoint that fails is
        // skipped for the rest of its interval, not retried at a later block, and the
        // record is persisted so a restart inside the interval does not undo the skip.
        self.record_checkpoint_attempt(block_time);

        let keep_n = platform_version.drive_abci.checkpoints.num_checkpoints as usize;

        // Build the checkpoint path: db_path/checkpoints/<block_height>
        let checkpoints_path = self.config.db_path.join("checkpoints");
        let checkpoint_path = checkpoints_path.join(block_height.to_string());

        let injected_fault = self.take_injected_checkpoint_fault();

        // Create the parent checkpoints directory if it doesn't exist
        fail_if_injected(injected_fault, CheckpointStep::CreateDirectory)
            .and_then(|()| std::fs::create_dir_all(&checkpoints_path))
            .map_err(|err| io_error(CheckpointStep::CreateDirectory, &checkpoints_path, err))?;

        // Create checkpoint DB (this creates the checkpoint_path directory). RocksDB
        // refuses an existing target, so the directory belongs to this attempt from
        // here on and is removed again if a later step fails.
        fail_if_injected(injected_fault, CheckpointStep::CreateCheckpoint)
            .map_err(|err| io_error(CheckpointStep::CreateCheckpoint, &checkpoint_path, err))?;
        self.drive.grove.create_checkpoint(&checkpoint_path)?;

        let checkpoint_db =
            match complete_checkpoint(&checkpoint_path, &platform_state, injected_fault) {
                Ok(checkpoint_db) => checkpoint_db,
                Err(error) => {
                    // Nothing references the directory yet. Remove it so a restart does
                    // not load a checkpoint that has no platform state next to it.
                    if let Err(cleanup_error) = std::fs::remove_dir_all(&checkpoint_path) {
                        tracing::warn!(
                            error = ?cleanup_error,
                            path = ?checkpoint_path,
                            "failed to remove incomplete grovedb checkpoint directory"
                        );
                    }
                    return Err(error);
                }
            };
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

    /// The step a test asked to fail on this attempt, consumed so the next attempt
    /// takes the next injected fault, or none.
    #[cfg(feature = "testing-config")]
    fn take_injected_checkpoint_fault(&self) -> Option<CheckpointStep> {
        self.config
            .testing_configs
            .checkpoint_faults
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
    }

    #[cfg(not(feature = "testing-config"))]
    fn take_injected_checkpoint_fault(&self) -> Option<CheckpointStep> {
        None
    }
}

/// Writes the platform state next to the checkpoint's database and opens the checkpoint.
fn complete_checkpoint(
    checkpoint_path: &Path,
    platform_state: &PlatformState,
    injected_fault: Option<CheckpointStep>,
) -> Result<GroveDb, Error> {
    // Save platform state to checkpoint directory on disk (after grovedb creates the directory)
    let checkpoint_state_path = checkpoint_path.join("platform_state.bin");
    let state_bytes = platform_state.serialize_to_bytes()?;
    fail_if_injected(injected_fault, CheckpointStep::WriteState)
        .and_then(|()| std::fs::write(&checkpoint_state_path, &state_bytes))
        .map_err(|err| io_error(CheckpointStep::WriteState, &checkpoint_state_path, err))?;

    // Open the checkpoint as a GroveDb instance
    fail_if_injected(injected_fault, CheckpointStep::OpenCheckpoint)
        .map_err(|err| io_error(CheckpointStep::OpenCheckpoint, checkpoint_path, err))?;
    Ok(GroveDb::open(checkpoint_path)?)
}

/// `Err` when a fault-injection test asked for `step` to fail, `Ok` otherwise.
fn fail_if_injected(
    injected_fault: Option<CheckpointStep>,
    step: CheckpointStep,
) -> std::io::Result<()> {
    if injected_fault == Some(step) {
        Err(std::io::Error::other(format!("injected {step:?} fault")))
    } else {
        Ok(())
    }
}

fn io_error(step: CheckpointStep, path: &Path, err: std::io::Error) -> Error {
    Error::Drive(IOErrorWithInfoString(
        err.into(),
        format!("{} at {}", step.description(), path.display()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::serialization::PlatformDeserializableFromVersionedStructureTrusted;
    use std::path::PathBuf;

    fn platform() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state()
    }

    fn committed_height(platform: &TempPlatform<MockCoreRPCLike>) -> u64 {
        platform.state.load().last_committed_block_height()
    }

    fn checkpoint_path(platform: &TempPlatform<MockCoreRPCLike>) -> PathBuf {
        platform
            .config
            .db_path
            .join("checkpoints")
            .join(committed_height(platform).to_string())
    }

    fn inject_fault(platform: &TempPlatform<MockCoreRPCLike>, step: CheckpointStep) {
        platform
            .config
            .testing_configs
            .checkpoint_faults
            .lock()
            .unwrap()
            .push_back(step);
    }

    fn create_checkpoint(platform: &TempPlatform<MockCoreRPCLike>) -> Result<(), Error> {
        platform.create_grovedb_checkpoint(PlatformVersion::latest())
    }

    fn assert_nothing_registered(platform: &TempPlatform<MockCoreRPCLike>, context: &str) {
        assert!(
            platform.drive.checkpoints.load().is_empty(),
            "{context}: a failed attempt must not register a checkpoint"
        );
        assert!(
            platform.checkpoint_platform_states.load().is_empty(),
            "{context}: a failed attempt must not register a platform state"
        );
    }

    /// The checkpoint for the committed height is registered in both caches and its
    /// platform state file round-trips the way startup loads it.
    fn assert_checkpoint_registered(platform: &TempPlatform<MockCoreRPCLike>, context: &str) {
        let height = committed_height(platform);

        let checkpoints = platform.drive.checkpoints.load();
        assert_eq!(
            checkpoints.keys().copied().collect::<Vec<_>>(),
            vec![height],
            "{context}"
        );

        let states = platform.checkpoint_platform_states.load();
        assert_eq!(
            states
                .get(&height)
                .map(|state| state.last_committed_block_height()),
            Some(height),
            "{context}"
        );

        let state_path = checkpoint_path(platform).join("platform_state.bin");
        let state_bytes = std::fs::read(&state_path)
            .unwrap_or_else(|error| panic!("{context}: {state_path:?}: {error}"));
        let state =
            PlatformState::versioned_deserialize_trusted(&state_bytes, PlatformVersion::latest())
                .expect("checkpoint platform state deserializes");
        assert_eq!(state.last_committed_block_height(), height, "{context}");
    }

    #[test]
    fn creates_checkpoint_with_platform_state() {
        let platform = platform();

        create_checkpoint(&platform).expect("checkpoint");

        assert_checkpoint_registered(&platform, "first checkpoint");
    }

    /// A regular file where the `checkpoints` directory should be makes directory
    /// creation fail with a real I/O error. Nothing is created, the obstacle is
    /// not ours to remove, and the attempt after it is cleared succeeds.
    #[test]
    fn directory_creation_failure_creates_nothing() {
        let platform = platform();
        let checkpoints_path = platform.config.db_path.join("checkpoints");
        std::fs::write(&checkpoints_path, b"not a directory").expect("write obstacle");

        let error = create_checkpoint(&platform).expect_err("directory creation must fail");

        assert!(
            matches!(
                &error,
                Error::Drive(IOErrorWithInfoString(_, info))
                    if info.starts_with(CheckpointStep::CreateDirectory.description())
            ),
            "{error:?}"
        );
        assert_nothing_registered(&platform, "directory creation failure");
        assert!(
            checkpoints_path.is_file(),
            "a path this attempt did not create is left alone"
        );

        std::fs::remove_file(&checkpoints_path).expect("remove obstacle");
        create_checkpoint(&platform).expect("retry once the obstacle is gone");

        assert_checkpoint_registered(&platform, "retry after directory creation failure");
    }

    /// RocksDB refuses to checkpoint into an existing directory. The directory is
    /// not this attempt's to remove, nothing is registered, and the attempt after
    /// the directory is cleared succeeds.
    #[test]
    fn checkpoint_creation_failure_leaves_existing_directory_alone() {
        let platform = platform();
        let checkpoint_path = checkpoint_path(&platform);
        std::fs::create_dir_all(&checkpoint_path).expect("create existing directory");
        let marker = checkpoint_path.join("marker");
        std::fs::write(&marker, b"stale").expect("write marker");

        let error = create_checkpoint(&platform).expect_err("checkpoint creation must fail");

        assert!(
            matches!(&error, Error::Drive(drive::error::Error::GroveDB(_))),
            "{error:?}"
        );
        assert_nothing_registered(&platform, "checkpoint creation failure");
        assert!(
            marker.is_file(),
            "a directory this attempt did not create is left alone"
        );

        std::fs::remove_dir_all(&checkpoint_path).expect("remove existing directory");
        create_checkpoint(&platform).expect("retry once the directory is gone");

        assert_checkpoint_registered(&platform, "retry after checkpoint creation failure");
    }

    /// Whichever step fails, the attempt registers nothing, leaves no checkpoint
    /// directory behind (removing the one it created when a later step fails), and
    /// the next attempt succeeds.
    #[test]
    fn injected_failure_at_any_step_leaves_nothing_behind_and_retry_succeeds() {
        for step in [
            CheckpointStep::CreateDirectory,
            CheckpointStep::CreateCheckpoint,
            CheckpointStep::WriteState,
            CheckpointStep::OpenCheckpoint,
        ] {
            let context = format!("{step:?}");
            let platform = platform();
            inject_fault(&platform, step);

            let error =
                create_checkpoint(&platform).expect_err("the injected fault must fail the attempt");

            assert!(
                matches!(
                    &error,
                    Error::Drive(IOErrorWithInfoString(_, info))
                        if info.starts_with(step.description())
                ),
                "{context}: {error:?}"
            );
            assert_nothing_registered(&platform, &context);
            assert!(
                !checkpoint_path(&platform).exists(),
                "{context}: no checkpoint directory may survive a failed attempt"
            );

            // The fault is consumed, so the retry runs the real steps
            create_checkpoint(&platform)
                .unwrap_or_else(|error| panic!("{context}: retry must succeed: {error:?}"));

            assert_checkpoint_registered(&platform, &context);
        }
    }
}
