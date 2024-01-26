use dpp::dashcore::hashes::Hash;
use dpp::dashcore::Txid;
use std::collections::BTreeMap;

use crate::{error::Error, platform_types::platform::Platform, rpc::core::CoreRPCLike};
use dashcore_rpc::json::AssetUnlockStatus;

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
            .get_asset_unlock_statuses(&withdrawal_indices)?;

        Ok(asset_unlock_statuses_result
            .into_iter()
            .zip(withdrawal_indices)
            // .filter(|(asset_unlock_status, _)| asset_unlock_status.is_some())
            .map(|(asset_unlock_status_result, withdrawal_index)| {
                // let asset_unlock_status_result = asset_unlock_status_result.unwrap();
                // Transaction has not been mined yet
                match asset_unlock_status_result.status {
                    AssetUnlockStatus::Chainlocked => (withdrawal_index, true),
                    _ => (withdrawal_index, false),
                }
                // if asset_unlock_status_result.status == AssetUnlockStatusResult < 0 {
                //     return (identifier, false);
                // };
                // let withdrawal_chain_locked = lock_result.chain_lock
                //     && current_chain_locked_core_height >= lock_result.height as u32;
                // (identifier, withdrawal_chain_locked)
            })
            .collect())
    }
}
