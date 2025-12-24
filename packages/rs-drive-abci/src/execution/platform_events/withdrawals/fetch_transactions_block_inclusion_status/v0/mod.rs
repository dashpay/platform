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
