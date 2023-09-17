use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

use crate::{error::Error, platform_types::platform::Platform, rpc::core::CoreRPCLike};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fetch Core transactions by range of Core heights
    pub(super) fn fetch_transactions_block_inclusion_status_v0<
        I: IntoIterator<Item = Identifier>,
    >(
        &self,
        current_chain_locked_core_height: u32,
        transaction_identifiers: I,
    ) -> Result<BTreeMap<Identifier, WithdrawalStatus>, Error> {
        transaction_identifiers.into_iter().map(|transaction_id| {
            let transaction_id_bytes: [u8; 32] = transaction_id.into_buffer();
            let extended_info = self
                .core_rpc
                .get_transaction_extended_info(transaction_id_bytes.into());
            match extended_info {
                Ok(extended_info) => {
                    if extended_info.chainlock {
                        WithdrawalStatus::COMPLETE
                    }
                }
                Err(e) => {}
            }
        })
    }
}
