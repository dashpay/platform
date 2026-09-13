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

        // Checkpoints are restore points for a running node. Replaying history,
        // ten minutes of chain time is a handful of blocks, so this fires dozens
        // of times a second and all but the last `keep_n` are deleted again
        // immediately — each one a RocksDB checkpoint over the whole database
        // plus a copy of the platform state. The node writes its first real
        // checkpoint once it reaches the tip.
        if crate::utils::is_historical_block(block_time) {
            return Ok(None);
        }

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

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is before the unix epoch")
            .as_millis() as u64
    }

    fn make_block_execution_context(height: u64, block_time_ms: u64) -> BlockExecutionContext {
        let platform_version = PlatformVersion::latest();
        let platform_state =
            crate::platform_types::platform_state::PlatformState::default_with_protocol_versions(
                platform_version.protocol_version,
                platform_version.protocol_version,
                &crate::config::PlatformConfig::default_for_network(
                    dpp::dashcore::Network::Testnet,
                ),
            )
            .expect("expected platform state");

        BlockExecutionContext::V0(BlockExecutionContextV0 {
            block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                height,
                round: 0,
                block_time_ms,
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
            block_platform_state: platform_state,
            proposer_results: None,
        })
    }

    #[test]
    fn test_first_block_should_always_checkpoint() {
        let platform_version = PlatformVersion::latest();
        let platform_config = crate::config::PlatformConfig {
            testing_configs: crate::config::PlatformTestConfig {
                disable_checkpoints: false,
                ..Default::default()
            },
            ..Default::default()
        };
        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // With no existing checkpoints and checkpoint feature enabled (v7+),
        // the first block should trigger a checkpoint
        if platform_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint
            .is_none()
        {
            // If checkpoints are disabled in this version, skip
            return;
        }

        // A block the network has just produced: checkpoints are restore points
        // for a running node, so the age of the block decides whether one is worth
        // taking, and a fixed fixture timestamp would read as ancient history.
        let block_execution_context = make_block_execution_context(1, now_ms());
        let result = platform
            .should_checkpoint_v0(&block_execution_context, platform_version)
            .expect("expected Ok");

        // When there are no checkpoints, it should return Some (checkpoint needed)
        assert!(result.is_some(), "first block should trigger checkpoint");
    }

    /// Replaying history, ten minutes of chain time is a handful of blocks, so a
    /// checkpoint would be taken dozens of times a second and all but the last
    /// few deleted again immediately. A node catching up takes none.
    #[test]
    fn test_historical_block_does_not_checkpoint() {
        let platform_version = PlatformVersion::latest();
        if platform_version
            .drive_abci
            .methods
            .block_end
            .should_checkpoint
            .is_none()
        {
            return;
        }

        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let block_execution_context =
            make_block_execution_context(1, now_ms() - 24 * 60 * 60 * 1000);
        let result = platform
            .should_checkpoint_v0(&block_execution_context, platform_version)
            .expect("expected Ok");

        assert!(
            result.is_none(),
            "a day-old block is being replayed, not followed"
        );
    }

    #[test]
    fn test_checkpoint_interval_zero_returns_none() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version.drive_abci.checkpoints.frequency_seconds = 0;

        let block_execution_context = make_block_execution_context(1, 1_000_000);
        let result = platform
            .should_checkpoint_v0(&block_execution_context, &modified_version)
            .expect("expected Ok");

        assert!(
            result.is_none(),
            "should return None when frequency is zero"
        );
    }

    #[test]
    fn test_num_checkpoints_zero_returns_none() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut modified_version = platform_version.clone();
        modified_version.drive_abci.checkpoints.num_checkpoints = 0;

        let block_execution_context = make_block_execution_context(1, 1_000_000);
        let result = platform
            .should_checkpoint_v0(&block_execution_context, &modified_version)
            .expect("expected Ok");

        assert!(
            result.is_none(),
            "should return None when num_checkpoints is zero"
        );
    }
}
