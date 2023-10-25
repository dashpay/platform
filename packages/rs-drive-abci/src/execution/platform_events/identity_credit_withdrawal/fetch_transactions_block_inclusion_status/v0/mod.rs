use dpp::dashcore::hashes::Hash;
use dpp::dashcore::Txid;
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
        transaction_identifiers: Vec<[u8; 32]>,
    ) -> Result<BTreeMap<[u8; 32], bool>, Error> {
        let tx_ids: Vec<Txid> = transaction_identifiers
            .iter()
            .map(|transaction_id| Txid::from_byte_array(*transaction_id))
            .collect();
        let transactions_are_chain_locked_result =
            self.core_rpc.get_transactions_are_chain_locked(tx_ids)?;

        Ok(transactions_are_chain_locked_result
            .into_iter()
            .zip(transaction_identifiers)
            .map(|(lock_result, identifier)| {
                let Some(mined_height) = lock_result.height else {
                    return (identifier, false);
                };
                let withdrawal_chain_locked =
                    lock_result.chain_lock && current_chain_locked_core_height >= mined_height;
                (identifier, withdrawal_chain_locked)
            })
            .collect())
    }
}
