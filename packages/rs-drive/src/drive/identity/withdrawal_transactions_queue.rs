// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! This module defines functions within the Drive struct related to withdrawal transaction (AssetUnlock)
//!

use std::borrow::Borrow;
use std::ops::RangeFull;

use dashcore::consensus::Encodable;
use dashcore::{Script, TxOut};
use dashcore::blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{AssetUnlockBaseTransactionInfo, AssetUnlockBasePayload};
use dashcore_rpc::RpcApi;

use dpp::contracts::withdrawals_contract;
use dpp::identity::convert_credits_to_satoshi;
use dpp::prelude::{DataContract, Document, Identifier};
use dpp::util::hash;
use dpp::util::json_value::JsonValueExt;
use dpp::util::string_encoding::Encoding;
use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{Element, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};

use serde_json::{json, Number, Value as JsonValue};

use crate::common;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;

/// constant id for transaction counter
pub const WITHDRAWAL_TRANSACTIONS_COUNTER_ID: [u8; 1] = [0];
/// constant id for subtree containing transactions queue
pub const WITHDRAWAL_TRANSACTIONS_QUEUE_ID: [u8; 1] = [1];

type WithdrawalTransaction = (Vec<u8>, Vec<u8>);

/// Add operations for creating initial withdrawal state structure
pub fn add_initial_withdrawal_state_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::WithdrawalTransactions as u8]);

    batch.add_insert(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_COUNTER_ID.to_vec(),
        Element::Item(0u64.to_be_bytes().to_vec(), None),
    );

    batch.add_insert_empty_tree(
        vec![vec![RootTree::WithdrawalTransactions as u8]],
        WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
    );
}

impl Drive {
    /// Get latest withdrawal index in a queue
    pub fn fetch_latest_withdrawal_transaction_index(
        &self,
        transaction: TransactionArg,
    ) -> Result<u64, Error> {
        let result = self
            .grove
            .get(
                [Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions).as_slice()],
                &WITHDRAWAL_TRANSACTIONS_COUNTER_ID,
                transaction,
            )
            .unwrap()
            .map_err(Error::GroveDB);

        if let Err(Error::GroveDB(grovedb::Error::PathKeyNotFound(_))) = &result {
            return Ok(0);
        }

        let element = result?;

        if let Element::Item(counter_bytes, _) = element {
            let counter = u64::from_be_bytes(counter_bytes.try_into().map_err(|_| {
                DriveError::CorruptedWithdrawalTransactionsCounterInvalidLength(
                    "withdrawal transactions counter must be an u64",
                )
            })?);

            Ok(counter)
        } else {
            Err(Error::Drive(
                DriveError::CorruptedWithdrawalTransactionsCounterNotItem(
                    "withdrawal transactions counter must be an item",
                ),
            ))
        }
    }

    fn build_withdrawal_transactions_from_documents(
        &self,
        documents: &mut Vec<Document>,
        data_contract: &DataContract,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<Vec<WithdrawalTransaction>, Error> {
        let mut withdrawals: Vec<(Vec<u8>, Vec<u8>)> = vec![];

        let latest_withdrawal_index =
            self.fetch_latest_withdrawal_transaction_index(transaction)?;

        for (i, mut document) in documents.iter_mut().enumerate() {
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

            withdrawals.push((
                transaction_index.to_be_bytes().to_vec(),
                transaction_buffer.clone(),
            ));

            let transacton_id = hash::hash(transaction_buffer);

            document
                .data
                .insert(
                    "transactionId".to_string(),
                    JsonValue::Array(
                        transacton_id
                            .into_iter()
                            .map(|byte| JsonValue::Number(Number::from(byte)))
                            .collect(),
                    ),
                )
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't update document field: transactionId",
                    ))
                })?;

            document
                .data
                .insert(
                    "status".to_string(),
                    JsonValue::Number(Number::from(withdrawals_contract::statuses::POOLED)),
                )
                .map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't update document field: status",
                    ))
                })?;

            document.revision += 1;

            self.update_document_for_contract_cbor(
                &document.to_cbor().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't cbor withdrawal document",
                    ))
                })?,
                &data_contract.to_cbor().map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't cbor withdrawal data contract",
                    ))
                })?,
                withdrawals_contract::types::WITHDRAWAL,
                Some(&document.owner_id.to_buffer()),
                block_time,
                true,
                StorageFlags { epoch: 1 },
                transaction,
            )?;
        }

        Ok(withdrawals)
    }

    ///
    pub fn pool_withdrawals_into_transactions(
        &self,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let (data_contract, _) = self.fetch_contract(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            transaction,
            self.cache.borrow_mut(),
        )?;

        let data_contract = data_contract.ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let mut documents = self.fetch_withdrawal_documents_by_status(
            withdrawals_contract::statuses::QUEUED,
            transaction,
        )?;

        let withdrawal_transactions = self.build_withdrawal_transactions_from_documents(
            &mut documents,
            data_contract.borrow(),
            block_time,
            transaction,
        )?;

        let mut batch = GroveDbOpBatch::new();

        self.add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawal_transactions);

        if batch.len() > 0 {
            self.grove_apply_batch(batch, true, transaction)?;
        }

        Ok(())
    }

    fn fetch_core_block_transactions(
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

    fn fetch_withdrawal_documents_by_status(
        &self,
        status: u8,
        transaction: TransactionArg,
    ) -> Result<Vec<Document>, Error> {
        let query_value = json!({
            "where": [
                ["status", "==", status],
            ],
            "orderBy": [
                ["$createdAt", "desc"],
            ]
        });

        let query_cbor = common::value_to_cbor(query_value, None);

        let (documents, _, _) = self.query_documents(
            &query_cbor,
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            withdrawals_contract::types::WITHDRAWAL,
            transaction,
        )?;

        let documents = documents
            .into_iter()
            .map(|document_cbor| {
                Document::from_cbor(document_cbor).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't create a document from cbor",
                    ))
                })
            })
            .collect::<Result<Vec<Document>, Error>>()?;

        Ok(documents)
    }

    ///
    pub fn update_withdrawal_statuses(
        &self,
        last_synced_core_height: u64,
        core_chain_locked_height: u64,
        block_time: f64,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let (data_contract, _) = self.fetch_contract(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            transaction,
            self.cache.borrow_mut(),
        )?;

        let data_contract = data_contract.ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let core_transactions =
            self.fetch_core_block_transactions(last_synced_core_height, core_chain_locked_height)?;

        let broadcasted_documents = self.fetch_withdrawal_documents_by_status(
            withdrawals_contract::statuses::BROADCASTED,
            transaction,
        )?;

        for mut document in broadcasted_documents {
            let transaction_sign_height =
                document
                    .data
                    .get_u64("transactionSignHeight")
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't get transactionSignHeight from withdrawal document",
                        ))
                    })?;

            let transaction_id_bytes = document.data.get_bytes("transactionId").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get transactionId from withdrawal document",
                ))
            })?;

            let transaction_id = hex::encode(transaction_id_bytes);

            if core_transactions.contains(&transaction_id)
                || core_chain_locked_height - transaction_sign_height > 48
            {
                let status = if core_transactions.contains(&transaction_id) {
                    withdrawals_contract::statuses::COMPLETE
                } else {
                    withdrawals_contract::statuses::EXPIRED
                };

                document
                    .data
                    .insert(
                        "status".to_string(),
                        JsonValue::Number(Number::from(status)),
                    )
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't update document field: status",
                        ))
                    })?;

                document.revision += 1;

                self.update_document_for_contract_cbor(
                    &document.to_cbor().map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't cbor withdrawal document",
                        ))
                    })?,
                    &data_contract.to_cbor().map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't cbor withdrawal data contract",
                        ))
                    })?,
                    withdrawals_contract::types::WITHDRAWAL,
                    Some(&document.owner_id.to_buffer()),
                    block_time,
                    true,
                    StorageFlags { epoch: 1 },
                    transaction,
                )?;
            }
        }

        Ok(())
    }

    /// Add counter update operations to the batch
    pub fn add_update_withdrawal_index_counter_operation(
        &self,
        batch: &mut GroveDbOpBatch,
        value: Vec<u8>,
    ) {
        batch.add_insert(
            vec![vec![RootTree::WithdrawalTransactions as u8]],
            WITHDRAWAL_TRANSACTIONS_COUNTER_ID.to_vec(),
            Element::Item(value, None),
        );
    }

    /// Add insert operations for withdrawal transactions to the batch
    pub fn add_enqueue_withdrawal_transaction_operations(
        &self,
        batch: &mut GroveDbOpBatch,
        withdrawals: Vec<(Vec<u8>, Vec<u8>)>,
    ) {
        for (id, bytes) in withdrawals {
            batch.add_insert(
                vec![
                    vec![RootTree::WithdrawalTransactions as u8],
                    WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
                ],
                id,
                Element::Item(bytes, None),
            );
        }
    }

    /// Get specified amount of withdrawal transactions from the DB
    pub fn dequeue_withdrawal_transactions(
        &self,
        num_of_transactions: u16,
        transaction: TransactionArg,
    ) -> Result<Vec<WithdrawalTransaction>, Error> {
        let mut query = Query::new();

        query.insert_item(QueryItem::RangeFull(RangeFull));

        let path_query = PathQuery {
            path: vec![
                vec![RootTree::WithdrawalTransactions as u8],
                WITHDRAWAL_TRANSACTIONS_QUEUE_ID.to_vec(),
            ],
            query: SizedQuery {
                query,
                limit: Some(num_of_transactions),
                offset: None,
            },
        };

        let result_items = self
            .grove
            .query_raw(&path_query, QueryKeyElementPairResultType, transaction)
            .unwrap()
            .map_err(Error::GroveDB)?
            .0
            .to_key_elements();

        let withdrawals = result_items
            .into_iter()
            .map(|(id, element)| match element {
                Element::Item(bytes, _) => Ok((id, bytes)),
                _ => Err(Error::Drive(DriveError::CorruptedWithdrawalNotItem(
                    "withdrawal is not an item",
                ))),
            })
            .collect::<Result<Vec<(Vec<u8>, Vec<u8>)>, Error>>()?;

        if !withdrawals.is_empty() {
            let mut batch_operations: Vec<DriveOperation> = vec![];
            let mut drive_operations: Vec<DriveOperation> = vec![];

            let withdrawals_path: [&[u8]; 2] = [
                Into::<&[u8; 1]>::into(RootTree::WithdrawalTransactions),
                &WITHDRAWAL_TRANSACTIONS_QUEUE_ID,
            ];

            for (id, _) in withdrawals.iter() {
                self.batch_delete(
                    withdrawals_path,
                    id,
                    true,
                    transaction,
                    &mut batch_operations,
                )?;
            }

            self.apply_batch_drive_operations(
                true,
                transaction,
                batch_operations,
                &mut drive_operations,
            )?;
        }

        Ok(withdrawals)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::common::helpers::setup::setup_system_data_contract;
    use dpp::tests::fixtures::get_withdrawals_data_contract_fixture;
    use dpp::{contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture};

    use crate::common::helpers::setup::setup_document;
    use crate::drive::batch::GroveDbOpBatch;
    use serde_json::json;

    use dashcore::BlockHash;

    use crate::rpc::core::MockCoreRPCLike;
    use dashcore::hashes::hex::FromHex;

    mod build_withdrawal_transactions_from_documents {
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
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                }),
            );

            setup_document(&drive, &document_1, &data_contract, Some(&transaction));

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                }),
            );

            setup_document(&drive, &document_2, &data_contract, Some(&transaction));

            let mut documents = vec![document_1, document_2];

            let transactions = drive
                .build_withdrawal_transactions_from_documents(
                    &mut documents,
                    &data_contract,
                    1f64,
                    Some(&transaction),
                )
                .expect("to build transactions from documents");

            assert_eq!(
                transactions,
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

            let updated_documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::POOLED,
                    Some(&transaction),
                )
                .expect("to fetch withdrawal documents");

            let tx_ids = [
                "73050b2f1cdc267ecd9ccd10038e4c957fc108a404704e83077a593787b5f122",
                "de7889314e9dcfc6f7b142c18acc3bd1ccbee5f37d525651cdb3d5ce7fe66700",
            ];

            for document in updated_documents {
                assert_eq!(document.revision, 2);

                let tx_id: Vec<u8> = document
                    .data
                    .get("transactionId")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|byte| byte.as_u64().unwrap() as u8)
                    .collect();

                let tx_id_hex = hex::encode(tx_id);

                assert!(tx_ids.contains(&tx_id_hex.as_str()));
            }
        }
    }

    mod fetch_withdrawal_documents_by_status {
        use super::*;

        #[test]
        fn test_return_list_of_documents() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::QUEUED,
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 0);

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::QUEUED,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::QUEUED,
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::POOLED,
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
        }
    }

    mod fetch_core_block_transactions {
        use dashcore::hashes::hex::ToHex;

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

    mod queue {
        use super::*;

        #[test]
        fn test_enqueue_and_dequeue() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let withdrawals: Vec<(Vec<u8>, Vec<u8>)> = (0..17)
                .map(|i: u8| (i.to_be_bytes().to_vec(), vec![i; 32]))
                .collect();

            let mut batch = GroveDbOpBatch::new();

            drive.add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawals);

            drive
                .grove_apply_batch(batch, true, Some(&transaction))
                .expect("to apply ops");

            let withdrawals = drive
                .dequeue_withdrawal_transactions(16, Some(&transaction))
                .expect("to dequeue withdrawals");

            assert_eq!(withdrawals.len(), 16);

            let withdrawals = drive
                .dequeue_withdrawal_transactions(16, Some(&transaction))
                .expect("to dequeue withdrawals");

            assert_eq!(withdrawals.len(), 1);

            let withdrawals = drive
                .dequeue_withdrawal_transactions(16, Some(&transaction))
                .expect("to dequeue withdrawals");

            assert_eq!(withdrawals.len(), 0);
        }
    }

    mod index {
        use super::*;

        #[test]
        fn test_withdrawal_transaction_counter() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let mut batch = GroveDbOpBatch::new();

            let counter: u64 = 42;

            drive.add_update_withdrawal_index_counter_operation(
                &mut batch,
                counter.to_be_bytes().to_vec(),
            );

            drive
                .grove_apply_batch(batch, false, Some(&transaction))
                .expect("to apply ops");

            let stored_counter = drive
                .fetch_latest_withdrawal_transaction_index(Some(&transaction))
                .expect("to withdraw counter");

            assert_eq!(stored_counter, counter);
        }

        #[test]
        fn test_returns_0_if_empty() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let stored_counter = drive
                .fetch_latest_withdrawal_transaction_index(Some(&transaction))
                .expect("to withdraw counter");

            assert_eq!(stored_counter, 0);
        }
    }
}
