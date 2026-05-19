use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::epoch_info::v0::EpochInfoV0Methods;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the drive cache at the end of finalize block. This does a few things like merging
    /// the data contract cache and the platform versions cache.
    ///
    #[inline(always)]
    pub(super) fn update_drive_cache_v0(&self, block_execution_context: &BlockExecutionContext) {
        // Update global cache with updated contracts
        self.drive
            .cache
            .data_contracts
            .merge_and_clear_block_cache();

        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();

        // Clear previously proposed versions since we started a new epoch
        // For more information read comments in `upgrade_protocol_version_v0` function
        if block_execution_context
            .epoch_info()
            .is_epoch_change_but_not_genesis()
        {
            protocol_versions_counter.clear_global_cache();
            protocol_versions_counter.unblock_global_cache();
        }

        // Update proposed versions with new proposal from the current block
        protocol_versions_counter.merge_block_cache()
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

    fn make_block_execution_context(epoch_info: EpochInfo) -> BlockExecutionContext {
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
                height: 1,
                round: 0,
                block_time_ms: 1_000_000,
                previous_block_time_ms: None,
                proposer_pro_tx_hash: [0u8; 32],
                core_chain_locked_height: 1,
                block_hash: None,
                app_hash: None,
            }),
            epoch_info,
            unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
            block_address_balance_changes: BTreeMap::new(),
            block_platform_state: platform_state,
            proposer_results: None,
        })
    }

    #[test]
    fn test_update_drive_cache_merges_block_cache_without_epoch_change() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Put some data in the block cache
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.set_block_cache_version_count(next_version, 5);
        }

        // No epoch change
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: false,
        });

        let block_execution_context = make_block_execution_context(epoch_info);

        platform.update_drive_cache_v0(&block_execution_context);

        // After merging, the block cache data should now be in the global cache
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            let value = counter.get(&next_version).expect("expected to get value");
            assert_eq!(value, Some(&5u64));
        }
    }

    #[test]
    fn test_update_drive_cache_clears_and_unblocks_global_cache_on_epoch_change() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Put some data in both caches and block the global cache
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 50);
            counter.set_block_cache_version_count(next_version, 3);
            counter.block_global_cache();
        }

        // Verify global cache is blocked
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            assert!(counter.get(&next_version).is_err());
        }

        // Epoch change (not genesis)
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 5,
            previous_epoch_index: Some(4),
            is_epoch_change: true,
        });

        let block_execution_context = make_block_execution_context(epoch_info);

        platform.update_drive_cache_v0(&block_execution_context);

        // After epoch change, global cache should be cleared and unblocked
        // Then block cache (with version 3) should be merged into the now-empty global cache
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            // Global cache was cleared, then block cache merged in
            let value = counter.get(&next_version).expect("expected to get value");
            assert_eq!(value, Some(&3u64));
        }
    }

    #[test]
    fn test_update_drive_cache_does_not_clear_global_cache_without_epoch_change() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;
        let another_version = platform_version.protocol_version + 2;

        // Put data in global cache and block cache
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 50);
            counter.set_block_cache_version_count(another_version, 10);
        }

        // Not an epoch change
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 5,
            previous_epoch_index: None,
            is_epoch_change: false,
        });

        let block_execution_context = make_block_execution_context(epoch_info);

        platform.update_drive_cache_v0(&block_execution_context);

        // Global cache should NOT be cleared - should still have the old data
        // And block cache should be merged in
        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            let v1 = counter.get(&next_version).expect("expected to get value");
            assert_eq!(v1, Some(&50u64));
            let v2 = counter
                .get(&another_version)
                .expect("expected to get value");
            assert_eq!(v2, Some(&10u64));
        }
    }

    #[test]
    fn test_update_drive_cache_contracts_cache_merge() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // Not an epoch change
        let epoch_info = EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: false,
        });

        let block_execution_context = make_block_execution_context(epoch_info);

        // This should not panic and should successfully merge the data contracts cache
        platform.update_drive_cache_v0(&block_execution_context);
    }
}
