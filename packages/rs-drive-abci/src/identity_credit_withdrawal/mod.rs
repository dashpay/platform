use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo,
    hashes::Hash, QuorumHash,
};
use dpp::{
    contracts::withdrawals_contract,
    prelude::{Document, Identifier},
    util::{hash, json_value::JsonValueExt, string_encoding::Encoding},
};
use drive::{
    drive::{
        batch::GroveDbOpBatch,
        identity::withdrawals::paths::get_withdrawal_transactions_expired_ids_path_as_u8,
    },
    fee_pools::epochs::Epoch,
    query::{Element, TransactionArg},
};
use serde_json::{Number, Value as JsonValue};

use crate::{
    error::{execution::ExecutionError, Error},
    platform::Platform,
};

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;

impl Platform {
    /// Update statuses for broadcasted withdrawals
    pub fn update_broadcasted_withdrawal_transaction_statuses(
        &self,
        last_synced_core_height: u64,
        core_chain_locked_height: u64,
        block_time_ms: u64,
        block_height: u64,
        current_epoch_index: u16,
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
            Some(&Epoch::new(current_epoch_index)),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let core_transactions = self
            .drive
            .fetch_core_block_transactions(last_synced_core_height, core_chain_locked_height)?;

        let broadcasted_documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::statuses::BROADCASTED,
            transaction,
        )?;

        for mut document in broadcasted_documents {
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
                || core_chain_locked_height - transaction_sign_height > 48
            {
                let status = if core_transactions.contains(&transaction_id) {
                    withdrawals_contract::statuses::COMPLETE
                } else {
                    let bytes = transaction_index.to_be_bytes();

                    let path = get_withdrawal_transactions_expired_ids_path_as_u8();

                    self.drive
                        .grove
                        .insert(
                            path,
                            &bytes,
                            Element::Item(bytes.to_vec(), None),
                            None,
                            transaction,
                        )
                        .unwrap()
                        .map_err(|_| {
                            Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "Can't update transaction_index in GroveDB",
                            ))
                        })?;

                    withdrawals_contract::statuses::EXPIRED
                };

                self.drive.update_document_data(
                    &contract_fetch_info.contract,
                    &mut document,
                    block_time_ms,
                    block_height,
                    current_epoch_index,
                    transaction,
                    |document: &mut Document| -> Result<&mut Document, drive::error::Error> {
                        document
                            .data
                            .insert(
                                "status".to_string(),
                                JsonValue::Number(Number::from(status)),
                            )
                            .map_err(|_| {
                                drive::error::Error::Drive(
                                    drive::error::drive::DriveError::CorruptedCodeExecution(
                                        "Can't update document field: status",
                                    ),
                                )
                            })?;

                        document.revision += 1;

                        Ok(document)
                    },
                )?;
            }
        }

        Ok(())
    }

    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub fn fetch_and_prepare_unsigned_withdrawal_transactions(
        &self,
        block_time_ms: u64,
        block_height: u64,
        current_epoch_index: u16,
        validator_set_quorum_hash: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<Vec<u8>>, Error> {
        // Get 16 latest withdrawal transactions from the queue
        let withdrawal_transactions = self
            .drive
            .dequeue_withdrawal_transactions(WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT, transaction)?;

        // Appending request_height and quorum_hash to withdrwal transaction
        // and pass it to JS Drive for singing and broadcasting
        withdrawal_transactions
            .into_iter()
            .map(|(_, bytes)| {
                let request_info = AssetUnlockRequestInfo {
                    request_height: block_height as u32,
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

                self.drive.update_document_transaction_id(
                    &original_transaction_id,
                    &update_transaction_id,
                    block_time_ms,
                    block_height,
                    current_epoch_index,
                    transaction,
                )?;

                Ok(bytes_buffer)
            })
            .collect::<Result<Vec<Vec<u8>>, Error>>()
    }

    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions(
        &self,
        block_time_ms: u64,
        block_height: u64,
        current_epoch_index: u16,
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
            Some(&Epoch::new(current_epoch_index)),
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

        for document in documents.iter_mut() {
            let transaction_id =
                hash::hash(withdrawal_transactions.get(&document.id).unwrap().1.clone());

            self.drive.update_document_data(
                &contract_fetch_info.contract,
                document,
                block_time_ms,
                block_height,
                current_epoch_index,
                transaction,
                |document: &mut Document| -> Result<&mut Document, drive::error::Error> {
                    document
                        .data
                        .insert(
                            "transactionId".to_string(),
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
                        .data
                        .insert(
                            "status".to_string(),
                            JsonValue::Number(Number::from(withdrawals_contract::statuses::POOLED)),
                        )
                        .map_err(|_| {
                            drive::error::Error::Drive(
                                drive::error::drive::DriveError::CorruptedCodeExecution(
                                    "Can't update document field: status",
                                ),
                            )
                        })?;

                    document.revision += 1;

                    Ok(document)
                },
            )?;
        }

        let mut batch = GroveDbOpBatch::new();

        let withdrawal_transactions = withdrawal_transactions
            .values()
            .into_iter()
            .cloned()
            .collect();

        self.drive
            .add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawal_transactions);

        if !batch.is_empty() {
            self.drive.grove_apply_batch(batch, true, transaction)?;
        }

        Ok(())
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

    mod update_withdrawal_statuses {

        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

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

            platform.drive.core_rpc = Some(Box::new(mock_rpc_client));

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
                    96,
                    1,
                    1,
                    1,
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
            let mut platform = setup_platform_with_initial_state_structure();

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
                .pool_withdrawals_into_transactions(1, 1, 1, Some(&transaction))
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
}
