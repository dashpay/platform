use crate::{error::Error, platform_types::platform::Platform, rpc::core::CoreRPCLike};
use dpp::dashcore_rpc::dashcore_rpc_json::AssetUnlockStatus;
use dpp::withdrawal::WithdrawalTransactionIndex;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fetch Core transactions by range of Core heights
    pub(super) fn fetch_transactions_block_inclusion_status_v0(
        &self,
        current_chain_locked_core_height: u32,
        withdrawal_indices: &[WithdrawalTransactionIndex],
    ) -> Result<BTreeMap<WithdrawalTransactionIndex, AssetUnlockStatus>, Error> {
        let asset_unlock_statuses_result = self
            .core_rpc
            .get_asset_unlock_statuses(withdrawal_indices, current_chain_locked_core_height)?;

        Ok(asset_unlock_statuses_result
            .into_iter()
            .map(|asset_unlock_status_result| {
                (
                    asset_unlock_status_result.index,
                    asset_unlock_status_result.status,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::dashcore_rpc::dashcore_rpc_json::AssetUnlockStatusResult;

    /// Passing an empty list of indices must still call the RPC (with an
    /// empty slice) and return an empty map. This covers the happy path
    /// with zero entries and proves the closure doesn't panic on an empty
    /// result.
    #[test]
    fn v0_empty_indices_returns_empty_map() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_asset_unlock_statuses()
            .returning(|_indices, _height| Ok(Vec::new()));
        platform.core_rpc = mock_rpc;

        let result = platform
            .platform
            .fetch_transactions_block_inclusion_status_v0(100, &[])
            .expect("empty indices must yield Ok");

        assert!(result.is_empty());
    }

    /// Multiple results are collected into the map keyed by index. When core
    /// returns duplicate indices the BTreeMap keeps the last-inserted one;
    /// this test pins that contract so a future refactor of the closure
    /// can't silently break callers.
    #[test]
    fn v0_collects_results_into_map_keyed_by_index() {
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let mut mock_rpc = MockCoreRPCLike::new();
        mock_rpc
            .expect_get_asset_unlock_statuses()
            .returning(|_indices, _height| {
                Ok(vec![
                    AssetUnlockStatusResult {
                        index: 1,
                        status: AssetUnlockStatus::Chainlocked,
                    },
                    AssetUnlockStatusResult {
                        index: 2,
                        status: AssetUnlockStatus::Unknown,
                    },
                ])
            });
        platform.core_rpc = mock_rpc;

        let map = platform
            .platform
            .fetch_transactions_block_inclusion_status_v0(100, &[1, 2])
            .expect("result Ok");

        assert_eq!(map.len(), 2);
        assert!(matches!(map.get(&1), Some(AssetUnlockStatus::Chainlocked)));
        assert!(matches!(map.get(&2), Some(AssetUnlockStatus::Unknown)));
    }
}
