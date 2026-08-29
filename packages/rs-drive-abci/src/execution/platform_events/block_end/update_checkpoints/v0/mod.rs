use crate::error::Error;
use crate::execution::platform_events::block_end::should_checkpoint::CheckpointNeededInfo;
use crate::execution::types::block_execution_context::BlockExecutionContext;
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
        // Check if we should create a checkpoint for this block
        let Some(CheckpointNeededInfo {
            block_height,
            block_time,
            current_checkpoints,
        }) = self.should_checkpoint(block_execution_context, platform_version)?
        else {
            return Ok(false);
        };

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

#[cfg(test)]
mod tests {
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_context(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
    ) -> BlockExecutionContext {
        let platform_state = platform.state.load();
        let block_platform_state = platform_state.as_ref().clone();

        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height: 1,
                round: 0,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info: EpochInfo::V0(EpochInfoV0 {
                current_epoch_index: 0,
                previous_epoch_index: None,
                is_epoch_change: false,
            }),
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state,
            proposer_results: None,
        })
    }

    /// With `disable_checkpoints = true` in testing config, `should_checkpoint`
    /// returns `None`, so `update_checkpoints_v0` must early-return `Ok(false)`
    /// without creating any checkpoint directory.
    #[test]
    fn v0_returns_false_when_checkpoints_disabled_in_test_config() {
        let platform_version = PlatformVersion::latest();
        let platform_config = crate::config::PlatformConfig {
            testing_configs: crate::config::PlatformTestConfig {
                disable_checkpoints: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let ctx = make_context(&platform);
        let got = platform
            .update_checkpoints_v0(&ctx, platform_version)
            .expect("must succeed when disabled");
        assert!(!got, "returns false when no checkpoint is needed");
    }

    /// If the platform version reports `should_checkpoint = None`, no checkpoint
    /// is ever scheduled — `update_checkpoints_v0` must short-circuit to
    /// `Ok(false)` without touching the filesystem.
    #[test]
    fn v0_returns_false_when_should_checkpoint_version_is_none() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint = None;

        let ctx = make_context(&platform);
        let got = platform
            .update_checkpoints_v0(&ctx, &modified_version)
            .expect("must succeed when should_checkpoint is None");
        assert!(!got);
    }

    /// Set `frequency_seconds = 0` — `should_checkpoint` returns `None`, so
    /// `update_checkpoints_v0` returns `Ok(false)`. This exercises the
    /// misconfiguration-tolerance branch from a higher level than
    /// `should_checkpoint`'s own tests.
    #[test]
    fn v0_returns_false_when_frequency_is_zero() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version.drive_abci.checkpoints.frequency_seconds = 0;

        let ctx = make_context(&platform);
        let got = platform
            .update_checkpoints_v0(&ctx, &modified_version)
            .expect("must succeed when frequency is zero");
        assert!(!got);
    }
}
