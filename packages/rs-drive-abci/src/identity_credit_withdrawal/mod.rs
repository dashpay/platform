use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo,
    hashes::Hash, QuorumHash,
};
use dpp::{
    contracts::withdrawals_contract,
    data_contract::extra::DriveContractExt,
    prelude::{Document, Identifier},
    util::{hash, json_value::JsonValueExt, string_encoding::Encoding},
};
use drive::{
    drive::{
        batch::DriveOperationType, block_info::BlockInfo,
        identity::withdrawals::paths::WithdrawalTransaction,
    },
    fee_pools::epochs::Epoch,
    query::TransactionArg,
};
use serde_json::{Number, Value as JsonValue};

use crate::{
    block::BlockExecutionContext,
    error::{execution::ExecutionError, Error},
    platform::Platform,
};

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;

impl Platform {
    /// Update statuses for broadcasted withdrawals
    pub fn update_broadcasted_withdrawal_transaction_statuses(
        &self,
        last_synced_core_height: u64,
        block_execution_context: &BlockExecutionContext,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let (_, maybe_data_contract) = self.drive.get_contract_with_fetch_info(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            Some(&Epoch::new(
                block_execution_context.epoch_info.current_epoch_index,
            )),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let core_transactions = self.fetch_core_block_transactions(
            last_synced_core_height,
            block_execution_context.block_info.core_chain_locked_height,
        )?;

        let mut broadcasted_documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::statuses::BROADCASTED,
            transaction,
        )?;

        let block_info = BlockInfo {
            time_ms: block_execution_context.block_info.block_time_ms,
            height: block_execution_context.block_info.block_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index),
        };

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        for document in broadcasted_documents.iter_mut() {
            let transaction_sign_height =
                document
                    .data
                    .get_u64("transactionSignHeight")
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't get transactionSignHeight from withdrawal document",
                        ))
                    })?;

            let transaction_id_bytes = document.data.get_bytes("transactionId").map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Can't get transactionId from withdrawal document",
                ))
            })?;

            let transaction_id = hex::encode(transaction_id_bytes);

            let transaction_index = document.data.get_u64("transactionIndex").map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Can't get transactionIdex from withdrawal document",
                ))
            })?;

            if core_transactions.contains(&transaction_id)
                || block_execution_context.block_info.core_chain_locked_height
                    - transaction_sign_height
                    > 48
            {
                let status = if core_transactions.contains(&transaction_id) {
                    withdrawals_contract::statuses::COMPLETE
                } else {
                    self.drive.add_insert_expired_index_operation(
                        transaction_index,
                        &mut drive_operations,
                    );

                    withdrawals_contract::statuses::EXPIRED
                };

                self.drive.update_document_data(
                    document,
                    &block_info,
                    |document: &mut Document| -> Result<&mut Document, drive::error::Error> {
                        document
                            .set("status", JsonValue::Number(Number::from(status)))
                            .map_err(|_| {
                                drive::error::Error::Drive(
                                    drive::error::drive::DriveError::CorruptedCodeExecution(
                                        "Can't update document field: status",
                                    ),
                                )
                            })?;

                        Ok(document)
                    },
                )?;
            }
        }

        let drive_documents = self
            .drive
            .convert_dpp_documents_to_drive_documents(broadcasted_documents.iter())?;

        self.drive.add_update_multiple_documents_operations(
            &drive_documents,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name("withdrawal")
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
        );

        if !drive_operations.is_empty() {
            self.drive
                .apply_drive_operations(drive_operations, true, &block_info, transaction)?;
        }

        Ok(())
    }

    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub fn fetch_and_prepare_unsigned_withdrawal_transactions(
        &self,
        block_execution_context: &BlockExecutionContext,
        validator_set_quorum_hash: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let (_, maybe_data_contract) = self.drive.get_contract_with_fetch_info(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            Some(&Epoch::new(
                block_execution_context.epoch_info.current_epoch_index,
            )),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let block_info = BlockInfo {
            time_ms: block_execution_context.block_info.block_time_ms,
            height: block_execution_context.block_info.block_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index),
        };

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        // Get 16 latest withdrawal transactions from the queue
        let withdrawal_transactions = self
            .drive
            .dequeue_withdrawal_transactions(WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT, transaction)?;

        // Appending request_height and quorum_hash to withdrwal transaction
        // and pass it to JS Drive for singing and broadcasting
        let transactions_and_documents = withdrawal_transactions
            .into_iter()
            .map(|(_, bytes)| {
                let request_info = AssetUnlockRequestInfo {
                    request_height: block_execution_context.block_info.core_chain_locked_height
                        as u32,
                    quorum_hash: QuorumHash::hash(&validator_set_quorum_hash),
                };

                let mut bytes_buffer = vec![];

                request_info
                    .consensus_append_to_base_encode(bytes.clone(), &mut bytes_buffer)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "could not add aditional request info to asset unlock transaction",
                        ))
                    })?;

                let original_transaction_id = hash::hash(bytes);
                let update_transaction_id = hash::hash(bytes_buffer.clone());

                let mut document = self
                    .drive
                    .find_document_by_transaction_id(&original_transaction_id, transaction)?;

                self.drive
                    .update_document_data(&mut document, &block_info, |doc| {
                        doc.set(
                            "transactionId",
                            JsonValue::Array(
                                update_transaction_id
                                    .iter()
                                    .map(|byte| JsonValue::Number(Number::from(*byte)))
                                    .collect(),
                            ),
                        )
                        .map_err(|_| {
                            drive::error::Error::Drive(
                                drive::error::drive::DriveError::CorruptedCodeExecution(
                                    "Can't set document field: transactionId",
                                ),
                            )
                        })?;

                        Ok(doc)
                    })?;

                Ok((bytes_buffer, document))
            })
            .collect::<Result<Vec<(Vec<u8>, Document)>, Error>>()?;

        let drive_documents = self.drive.convert_dpp_documents_to_drive_documents(
            transactions_and_documents
                .iter()
                .map(|(_, document)| document),
        )?;

        self.drive.add_update_multiple_documents_operations(
            &drive_documents,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name("withdrawal")
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "could not get document type",
                    ))
                })?,
            &mut drive_operations,
        );

        if !drive_operations.is_empty() {
            self.drive
                .apply_drive_operations(drive_operations, true, &block_info, transaction)?;
        }

        Ok(transactions_and_documents
            .iter()
            .map(|(bytes, _)| bytes.clone())
            .collect())
    }

    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions_queue(
        &self,
        block_execution_context: &BlockExecutionContext,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let (_, maybe_data_contract) = self.drive.get_contract_with_fetch_info(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            Some(&Epoch::new(
                block_execution_context.epoch_info.current_epoch_index,
            )),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let mut documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::statuses::QUEUED,
            transaction,
        )?;

        let withdrawal_transactions = self
            .drive
            .build_withdrawal_transactions_from_documents(&documents, transaction)?;

        let block_info = BlockInfo {
            time_ms: block_execution_context.block_info.block_time_ms,
            height: block_execution_context.block_info.block_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index),
        };

        let mut drive_operations = vec![];

        for document in documents.iter_mut() {
            let transaction_id =
                hash::hash(withdrawal_transactions.get(&document.id).unwrap().1.clone());

            self.drive.update_document_data(
                document,
                &block_info,
                |document: &mut Document| -> Result<&mut Document, drive::error::Error> {
                    document
                        .set(
                            "transactionId",
                            JsonValue::Array(
                                transaction_id
                                    .clone()
                                    .into_iter()
                                    .map(|byte| JsonValue::Number(Number::from(byte)))
                                    .collect(),
                            ),
                        )
                        .map_err(|_| {
                            drive::error::Error::Drive(
                                drive::error::drive::DriveError::CorruptedCodeExecution(
                                    "Can't update document field: transactionId",
                                ),
                            )
                        })?;

                    document
                        .set(
                            "status",
                            JsonValue::Number(Number::from(withdrawals_contract::statuses::POOLED)),
                        )
                        .map_err(|_| {
                            drive::error::Error::Drive(
                                drive::error::drive::DriveError::CorruptedCodeExecution(
                                    "Can't update document field: status",
                                ),
                            )
                        })?;

                    Ok(document)
                },
            )?;
        }

        let drive_documents = documents
            .iter()
            .map(|document| {
                drive::contract::document::Document::from_cbor(
                    &document.to_cbor().map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't fetch withdrawal data contract",
                        ))
                    })?,
                    None,
                    None,
                )
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })
            })
            .collect::<Result<Vec<drive::contract::document::Document>, Error>>()?;

        self.drive.add_update_multiple_documents_operations(
            &drive_documents,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name("withdrawal")
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
        );

        let withdrawal_transactions: Vec<WithdrawalTransaction> = withdrawal_transactions
            .values()
            .into_iter()
            .cloned()
            .collect();

        self.drive.add_enqueue_withdrawal_transaction_operations(
            &withdrawal_transactions,
            &mut drive_operations,
        );

        if !drive_operations.is_empty() {
            self.drive
                .apply_drive_operations(drive_operations, true, &block_info, transaction)?;
        }

        Ok(())
    }

    /// Fetch Core transactions by range of Core heights
    pub fn fetch_core_block_transactions(
        &self,
        last_synced_core_height: u64,
        core_chain_locked_height: u64,
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
    use dashcore::{
        hashes::hex::{FromHex, ToHex},
        BlockHash,
    };
    use dpp::{
        contracts::withdrawals_contract,
        tests::fixtures::{get_withdrawal_document_fixture, get_withdrawals_data_contract_fixture},
    };
    use drive::{common::helpers::setup::setup_document, rpc::core::MockCoreRPCLike};
    use serde_json::json;

    use crate::common::helpers::setup::setup_system_data_contract;

    use crate::common::helpers::setup::setup_platform_with_initial_state_structure;
    use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

    use crate::{
        block::{BlockExecutionContext, BlockInfo},
        execution::fee_pools::epoch::EpochInfo,
    };

    mod update_withdrawal_statuses {
        use dpp::prelude::DataContract;

        use super::*;

        #[test]
        fn test_statuses_are_updated() {
            let mut platform = setup_platform_with_initial_state_structure();

            let mut mock_rpc_client = MockCoreRPCLike::new();

            mock_rpc_client
                .expect_get_block_hash()
                .withf(|height| *height == 95)
                .returning(|_| {
                    Ok(BlockHash::from_hex(
                        "0000000000000000000000000000000000000000000000000000000000000000",
                    )
                    .unwrap())
                });

            mock_rpc_client
                .expect_get_block_hash()
                .withf(|height| *height == 96)
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
                        "tx": ["0101010101010101010101010101010101010101010101010101010101010101"]
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
                        "tx": ["0202020202020202020202020202020202020202020202020202020202020202"]
                    }))
                });

            platform.core_rpc = Box::new(mock_rpc_client);

            let transaction = platform.drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            // TODO: figure out the bug in data contract factory
            let data_contract = DataContract::from_cbor(
                data_contract
                    .to_cbor()
                    .expect("to convert contract to CBOR"),
            )
            .expect("to create data contract from CBOR");

            setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::BROADCASTED,
                    "transactionIndex": 1,
                    "transactionSignHeight": 93,
                    "transactionId": vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                }),
            );

            setup_document(
                &platform.drive,
                &document_1,
                &data_contract,
                Some(&transaction),
            );

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::BROADCASTED,
                    "transactionIndex": 2,
                    "transactionSignHeight": 10,
                    "transactionId": vec![3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                }),
            );

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                Some(&transaction),
            );

            platform
                .update_broadcasted_withdrawal_transaction_statuses(
                    95,
                    &BlockExecutionContext {
                        block_info: BlockInfo {
                            block_height: 1,
                            block_time_ms: 1,
                            previous_block_time_ms: Some(1),
                            proposer_pro_tx_hash: [
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ],
                            core_chain_locked_height: 96,
                        },
                        epoch_info: EpochInfo {
                            current_epoch_index: 1,
                            previous_epoch_index: None,
                            is_epoch_change: false,
                        },
                    },
                    Some(&transaction),
                )
                .expect("to update withdrawal statuses");

            let documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::EXPIRED,
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
            assert_eq!(documents.get(0).unwrap().id, document_2.id);

            let documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::COMPLETE,
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
            assert_eq!(documents.get(0).unwrap().id, document_1.id);
        }
    }

    mod pool_withdrawals_into_transactions {

        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

        use super::*;

        #[test]
        fn test_pooling() {
            let platform = setup_platform_with_initial_state_structure();

            let transaction = platform.drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::QUEUED,
                    "transactionIndex": 1,
                }),
            );

            setup_document(
                &platform.drive,
                &document_1,
                &data_contract,
                Some(&transaction),
            );

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::QUEUED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                Some(&transaction),
            );

            platform
                .pool_withdrawals_into_transactions_queue(
                    &BlockExecutionContext {
                        block_info: BlockInfo {
                            block_height: 1,
                            block_time_ms: 1,
                            previous_block_time_ms: Some(1),
                            proposer_pro_tx_hash: [
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            ],
                            core_chain_locked_height: 96,
                        },
                        epoch_info: EpochInfo {
                            current_epoch_index: 1,
                            previous_epoch_index: None,
                            is_epoch_change: false,
                        },
                    },
                    Some(&transaction),
                )
                .expect("to pool withdrawal documents into transactions");

            let updated_documents = platform
                .drive
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

    mod fetch_core_block_transactions {

        use super::*;

        #[test]
        fn test_fetches_core_transactions() {
            let mut platform = setup_platform_with_initial_state_structure();

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

            platform.core_rpc = Box::new(mock_rpc_client);

            let transactions = platform
                .fetch_core_block_transactions(1, 2)
                .expect("to fetch core transactions");

            assert_eq!(transactions.len(), 2);
            assert_eq!(transactions, ["1", "2"]);
        }
    }
}
