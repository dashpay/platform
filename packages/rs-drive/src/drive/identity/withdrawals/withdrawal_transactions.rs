use std::{collections::HashMap, ops::RangeFull};

use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    },
    consensus::Encodable,
    Script, TxOut,
};
use dpp::{
    identity::convert_credits_to_satoshi,
    prelude::{Document, Identifier},
    util::json_value::JsonValueExt,
};
use grovedb::TransactionArg;
use serde_json::Value as JsonValue;

use crate::{
    drive::Drive,
    error::{drive::DriveError, Error},
};

use super::paths::WithdrawalTransaction;

impl Drive {
    /// Build list of Core transactions from withdrawal documents
    pub fn build_withdrawal_transactions_from_documents(
        &self,
        documents: &[Document],
        transaction: TransactionArg,
    ) -> Result<HashMap<Identifier, WithdrawalTransaction>, Error> {
        let mut withdrawals: HashMap<Identifier, WithdrawalTransaction> = HashMap::new();

        let latest_withdrawal_index =
            self.fetch_latest_withdrawal_transaction_index(transaction)?;

        for (i, document) in documents.iter().enumerate() {
            let output_script = document.data.get_bytes("outputScript").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get outputScript from withdrawal document",
                ))
            })?;

            let amount = document.data.get_u64("amount").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get amount from withdrawal document",
                ))
            })?;

            let core_fee_per_byte = document.data.get_u64("coreFeePerByte").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get coreFeePerByte from withdrawal document",
                ))
            })?;

            let state_transition_size = 184;

            let output_script: Script = Script::from(output_script);

            let tx_out = TxOut {
                value: convert_credits_to_satoshi(amount),
                script_pubkey: output_script,
            };

            let transaction_index = latest_withdrawal_index + i as u64;

            let withdrawal_transaction = AssetUnlockBaseTransactionInfo {
                version: 1,
                lock_time: 0,
                output: vec![tx_out],
                base_payload: AssetUnlockBasePayload {
                    version: 1,
                    index: transaction_index,
                    fee: (state_transition_size * core_fee_per_byte * 1000) as u32,
                },
            };

            let mut transaction_buffer: Vec<u8> = vec![];

            withdrawal_transaction
                .consensus_encode(&mut transaction_buffer)
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't consensus encode a withdrawal transaction",
                    ))
                })?;

            withdrawals.insert(
                document.id.clone(),
                (
                    transaction_index.to_be_bytes().to_vec(),
                    transaction_buffer.clone(),
                ),
            );
        }

        Ok(withdrawals)
    }

    /// Fetch Core transactions by range of Core heights
    pub fn fetch_core_block_transactions(
        &self,
        last_synced_core_height: u64,
        core_chain_locked_height: u64,
    ) -> Result<Vec<String>, Error> {
        let core_rpc =
            self.core_rpc
                .as_ref()
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "Core RPC client has not been set up",
                )))?;

        let mut tx_hashes: Vec<String> = vec![];

        for height in last_synced_core_height..=core_chain_locked_height {
            let block_hash = core_rpc.get_block_hash(height).map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "could not get block by height",
                ))
            })?;

            let block_json: JsonValue = core_rpc.get_block_json(&block_hash).map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "could not get block by hash",
                ))
            })?;

            if let Some(transactions) = block_json.get("tx") {
                if let Some(transactions) = transactions.as_array() {
                    for transaction_hash in transactions {
                        tx_hashes.push(
                            transaction_hash
                                .as_str()
                                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
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
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use dashcore::{hashes::hex::FromHex, hashes::hex::ToHex, BlockHash};
    use serde_json::json;

    use crate::rpc::core::MockCoreRPCLike;

    use dpp::{
        contracts::withdrawals_contract,
        tests::fixtures::{get_withdrawal_document_fixture, get_withdrawals_data_contract_fixture},
    };

    use crate::{
        common::helpers::setup::{setup_document, setup_system_data_contract},
        drive::identity::withdrawals::paths::WithdrawalTransaction,
    };

    mod fetch_core_block_transactions {

        use super::*;

        #[test]
        fn test_fetches_core_transactions() {
            let mut drive = setup_drive_with_initial_state_structure();

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
                    bh.to_hex()
                        == "0000000000000000000000000000000000000000000000000000000000000000"
                })
                .returning(|_| {
                    Ok(json!({
                        "tx": ["1"]
                    }))
                });

            mock_rpc_client
                .expect_get_block_json()
                .withf(|bh| {
                    bh.to_hex()
                        == "1111111111111111111111111111111111111111111111111111111111111111"
                })
                .returning(|_| {
                    Ok(json!({
                        "tx": ["2"]
                    }))
                });

            drive.core_rpc = Some(Box::new(mock_rpc_client));

            let transactions = drive
                .fetch_core_block_transactions(1, 2)
                .expect("to fetch core transactions");

            assert_eq!(transactions.len(), 2);
            assert_eq!(transactions, ["1", "2"]);
        }
    }

    mod build_withdrawal_transactions_from_documents {
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

        use super::*;

        #[test]
        fn test_build() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 1,
                }),
            );

            setup_document(&drive, &document_1, &data_contract, Some(&transaction));

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document_2, &data_contract, Some(&transaction));

            let documents = vec![document_1, document_2];

            let transactions = drive
                .build_withdrawal_transactions_from_documents(&documents, Some(&transaction))
                .expect("to build transactions from documents");

            assert_eq!(
                transactions
                    .values()
                    .cloned()
                    .collect::<Vec<WithdrawalTransaction>>(),
                vec![
                    (
                        vec![0, 0, 0, 0, 0, 0, 0, 0],
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 0, 0, 0, 0, 0, 0, 0, 0, 192, 206, 2, 0,
                        ],
                    ),
                    (
                        vec![0, 0, 0, 0, 0, 0, 0, 1],
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 1, 0, 0, 0, 0, 0, 0, 0, 192, 206, 2, 0,
                        ],
                    ),
                ],
            );
        }
    }
}
