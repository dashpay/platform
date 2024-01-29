use std::collections::BTreeMap;

use crate::{error::Error, platform_types::platform::Platform, rpc::core::CoreRPCLike};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fetch Core transactions by range of Core heights
    pub(super) fn fetch_transactions_block_inclusion_status_v0(
        &self,
        current_chain_locked_core_height: u32,
        withdrawal_indices: Vec<u64>,
    ) -> Result<BTreeMap<u64, bool>, Error> {
        let asset_unlock_statuses_result = self
            .core_rpc
            .get_asset_unlock_statuses(&withdrawal_indices, current_chain_locked_core_height)?;

        Ok(asset_unlock_statuses_result
            .into_iter()
            .zip(withdrawal_indices)
            .map(|(asset_unlock_status_result, withdrawal_index)| {
                (withdrawal_index, asset_unlock_status_result.is_mined)
            })
            .collect())
    }
}
