use dpp::dashcore::hashes::Hash;
use dpp::dashcore::Txid;
use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::prelude::Identifier;
use sha2::Sha256;
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
        transaction_identifiers: Vec<Identifier>,
    ) -> Result<BTreeMap<Identifier, bool>, Error> {
        let tx_ids: Vec<Txid> = transaction_identifiers
            .iter()
            .map(|transaction_id| {
                let transaction_id_bytes: [u8; 32] = transaction_id.into_buffer();
                Txid::from_byte_array(transaction_id_bytes)
            })
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
                    if lock_result.chain_lock && current_chain_locked_core_height >= mined_height {
                        true
                    } else {
                        false
                    };
                (identifier, withdrawal_chain_locked)
            })
            .collect())
    }
}
