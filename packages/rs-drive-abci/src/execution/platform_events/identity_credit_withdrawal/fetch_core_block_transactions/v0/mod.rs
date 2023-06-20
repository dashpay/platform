use std::{collections::HashMap, ops::Deref};

use dashcore_rpc::dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::{
        request_info::AssetUnlockRequestInfo,
        unqualified_asset_unlock::{AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo},
    },
    consensus::Encodable,
    hashes::Hash,
    QuorumHash, Script, TxOut,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::document::Document;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use drive::dpp::contracts::withdrawals_contract;
use drive::dpp::identifier::Identifier;
use drive::dpp::identity::convert_credits_to_satoshi;
use drive::dpp::util::hash;
use drive::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
use drive::grovedb::Transaction;
use drive::{drive::batch::DriveOperation, query::TransactionArg};
use serde_json::Value as JsonValue;

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Fetch Core transactions by range of Core heights
    pub fn fetch_core_block_transactions_v0(
        &self,
        last_synced_core_height: u32,
        core_chain_locked_height: u32,
    ) -> Result<Vec<String>, Error> {
        let mut tx_hashes: Vec<String> = vec![];

        for height in last_synced_core_height..=core_chain_locked_height {
            let block_hash = self.core_rpc.get_block_hash(height).map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "could not get block by height",
                ))
            })?;

            let block_json: JsonValue =
                self.core_rpc.get_block_json(&block_hash).map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "could not get block by hash",
                    ))
                })?;

            if let Some(transactions) = block_json.get("tx") {
                if let Some(transactions) = transactions.as_array() {
                    for transaction_hash in transactions {
                        tx_hashes.push(
                            transaction_hash
                                .as_str()
                                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                                    "could not get transaction hash as string",
                                )))?
                                .to_string(),
                        );
                    }
                }
            }
        }

        Ok(tx_hashes)
    }
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore::{
        hashes::hex::{FromHex, ToHex},
        BlockHash,
    };
    use dpp::{contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture};
    use drive::tests::helpers::setup::setup_document;
    use serde_json::json;

    use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

    use crate::test::helpers::setup::TestPlatformBuilder;

    use super::*;
    use crate::rpc::core::MockCoreRPCLike;

    #[test]
    fn test_fetches_core_transactions() {
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut mock_rpc_client = MockCoreRPCLike::new();

        mock_rpc_client
            .expect_get_block_hash()
            .withf(|height| *height == 1)
            .returning(|_| {
                Ok(BlockHash::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .unwrap())
            });

        mock_rpc_client
            .expect_get_block_hash()
            .withf(|height| *height == 2)
            .returning(|_| {
                Ok(BlockHash::from_hex(
                    "1111111111111111111111111111111111111111111111111111111111111111",
                )
                .unwrap())
            });

        mock_rpc_client
            .expect_get_block_json()
            .withf(|bh| {
                bh.to_hex() == "0000000000000000000000000000000000000000000000000000000000000000"
            })
            .returning(|_| {
                Ok(json!({
                    "tx": ["1"]
                }))
            });

        mock_rpc_client
            .expect_get_block_json()
            .withf(|bh| {
                bh.to_hex() == "1111111111111111111111111111111111111111111111111111111111111111"
            })
            .returning(|_| {
                Ok(json!({
                    "tx": ["2"]
                }))
            });

        platform.core_rpc = mock_rpc_client;

        let transactions = platform
            .fetch_core_block_transactions_v0(1, 2)
            .expect("to fetch core transactions");

        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions, ["1", "2"]);
    }
}
