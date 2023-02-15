use std::{collections::HashMap, ops::Deref};

use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::{
        request_info::AssetUnlockRequestInfo,
        unqualified_asset_unlock::{AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo},
    },
    consensus::Encodable,
    hashes::Hash,
    QuorumHash, Script, TxOut,
};
use drive::dpp::contracts::withdrawals_contract;
use drive::dpp::data_contract::DriveContractExt;
use drive::dpp::document::document_stub::DocumentStub;
use drive::dpp::identifier::Identifier;
use drive::dpp::identity::convert_credits_to_satoshi;
use drive::dpp::util::hash;
use drive::{
    drive::{
        batch::DriveOperationType, block_info::BlockInfo,
        identity::withdrawals::paths::WithdrawalTransaction,
    },
    fee_pools::epochs::Epoch,
    query::TransactionArg,
};
use serde_json::Value as JsonValue;

use crate::{
    block::BlockExecutionContext,
    error::{execution::ExecutionError, Error},
    platform::Platform,
};

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;
const NUMBER_OF_BLOCKS_BEFORE_EXPIRED: u64 = 48;

impl Platform {
    /// Update statuses for broadcasted withdrawals
    pub fn update_broadcasted_withdrawal_transaction_statuses(
        &self,
        last_synced_core_height: u64,
        block_execution_context: &BlockExecutionContext,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let data_contract_id = &withdrawals_contract::CONTRACT_ID;

        let (_, maybe_data_contract) = self.drive.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
            Some(&Epoch::new(
                block_execution_context.epoch_info.current_epoch_index,
            )),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("can't fetch withdrawal data contract"),
        ))?;

        let core_transactions = self.fetch_core_block_transactions(
            last_synced_core_height,
            block_execution_context.block_info.core_chain_locked_height,
        )?;

        let mut broadcasted_documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::Status::BROADCASTED.into(),
            transaction,
        )?;

        let block_info = BlockInfo {
            time_ms: block_execution_context.block_info.block_time_ms,
            height: block_execution_context.block_info.block_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index),
        };

        let mut drive_operations: Vec<DriveOperationType> = vec![];

        for document in broadcasted_documents.iter_mut() {
            let transaction_sign_height = document
                .get_u64(withdrawals_contract::property_names::TRANSACTION_SIGN_HEIGHT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get transactionSignHeight from withdrawal document",
                    ))
                })?;

            let transaction_id_bytes = document
                .get_bytes(withdrawals_contract::property_names::TRANSACTION_ID)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get transactionId from withdrawal document",
                    ))
                })?;

            let transaction_id = hex::encode(transaction_id_bytes);

            let transaction_index = document
                .get_u64(withdrawals_contract::property_names::TRANSACTION_INDEX)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get transactionIdex from withdrawal document",
                    ))
                })?;

            if core_transactions.contains(&transaction_id)
                || block_execution_context.block_info.core_chain_locked_height
                    - transaction_sign_height
                    > NUMBER_OF_BLOCKS_BEFORE_EXPIRED
            {
                let status = if core_transactions.contains(&transaction_id) {
                    withdrawals_contract::Status::COMPLETE
                } else {
                    self.drive.add_insert_expired_index_operation(
                        transaction_index,
                        &mut drive_operations,
                    );

                    withdrawals_contract::Status::EXPIRED
                };

                document.set_u8(withdrawals_contract::property_names::STATUS, status.into());

                document.set_i64(
                    withdrawals_contract::property_names::UPDATED_AT,
                    block_info.time_ms.try_into().map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't convert u64 block time to i64 updated_at",
                        ))
                    })?,
                );

                document.increment_revision().map_err(Error::Protocol)?;
            }
        }

        self.drive.add_update_multiple_documents_operations(
            &broadcasted_documents,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
        );

        self.drive
            .apply_drive_operations(drive_operations, true, &block_info, transaction)?;

        Ok(())
    }

    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub fn fetch_and_prepare_unsigned_withdrawal_transactions(
        &self,
        block_execution_context: &BlockExecutionContext,
        validator_set_quorum_hash: [u8; 32],
        transaction: TransactionArg,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let (_, maybe_data_contract) = self.drive.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
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
        let withdrawal_transactions = self.drive.dequeue_withdrawal_transactions(
            WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT,
            transaction,
            &mut drive_operations,
        )?;

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

                document.set_bytes(
                    withdrawals_contract::property_names::TRANSACTION_ID,
                    update_transaction_id,
                );

                document.set_i64(
                    withdrawals_contract::property_names::UPDATED_AT,
                    block_info.time_ms.try_into().map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't convert u64 block time to i64 updated_at",
                        ))
                    })?,
                );

                document.increment_revision().map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Could not increment document revision",
                    ))
                })?;

                Ok((bytes_buffer, document))
            })
            .collect::<Result<Vec<(Vec<u8>, DocumentStub)>, Error>>()?;

        let withdrawals = transactions_and_documents
            .iter()
            .map(|(bytes, _)| bytes.clone())
            .collect();

        let documents: Vec<DocumentStub> = transactions_and_documents
            .into_iter()
            .map(|(_, document)| document)
            .collect();

        self.drive.add_update_multiple_documents_operations(
            &documents,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)
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

        Ok(withdrawals)
    }

    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions_queue(
        &self,
        block_execution_context: &BlockExecutionContext,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let (_, maybe_data_contract) = self.drive.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
            Some(&Epoch::new(
                block_execution_context.epoch_info.current_epoch_index,
            )),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Execution(
            ExecutionError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let mut documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::Status::QUEUED.into(),
            transaction,
        )?;

        let withdrawal_transactions =
            self.build_withdrawal_transactions_from_documents(&documents, transaction)?;

        let block_info = BlockInfo {
            time_ms: block_execution_context.block_info.block_time_ms,
            height: block_execution_context.block_info.block_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index),
        };

        let mut drive_operations = vec![];

        for document in documents.iter_mut() {
            let document_id = Identifier::from_bytes(&document.id)?;

            let transaction_id =
                hash::hash(withdrawal_transactions.get(&document_id).unwrap().1.clone());

            document.set_bytes(
                withdrawals_contract::property_names::TRANSACTION_ID,
                transaction_id.clone(),
            );

            document.set_u8(
                withdrawals_contract::property_names::STATUS,
                withdrawals_contract::Status::POOLED as u8,
            );

            document.set_i64(
                withdrawals_contract::property_names::UPDATED_AT,
                block_info.time_ms.try_into().map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't convert u64 block time to i64 updated_at",
                    ))
                })?,
            );

            document.increment_revision().map_err(|_| {
                Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "Could not increment document revision",
                ))
            })?;
        }

        self.drive.add_update_multiple_documents_operations(
            &documents,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)
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

    /// Build list of Core transactions from withdrawal documents
    pub fn build_withdrawal_transactions_from_documents(
        &self,
        documents: &[DocumentStub],
        transaction: TransactionArg,
    ) -> Result<HashMap<Identifier, WithdrawalTransaction>, Error> {
        let mut withdrawals: HashMap<Identifier, WithdrawalTransaction> = HashMap::new();

        let latest_withdrawal_index = self
            .drive
            .remove_latest_withdrawal_transaction_index(transaction)?;

        for (i, document) in documents.iter().enumerate() {
            let output_script_bytes = document
                .get_bytes(withdrawals_contract::property_names::OUTPUT_SCRIPT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get outputScript from withdrawal document",
                    ))
                })?;

            let amount = document
                .get_u64(withdrawals_contract::property_names::AMOUNT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get amount from withdrawal document",
                    ))
                })?;

            let core_fee_per_byte = document
                .get_u64(withdrawals_contract::property_names::CORE_FEE_PER_BYTE)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get coreFeePerByte from withdrawal document",
                    ))
                })?;

            let state_transition_size = 190;

            let output_script: Script = Script::from(output_script_bytes);

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
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't consensus encode a withdrawal transaction",
                    ))
                })?;

            withdrawals.insert(
                Identifier::from_bytes(&document.id)?,
                (
                    transaction_index.to_be_bytes().to_vec(),
                    transaction_buffer.clone(),
                ),
            );
        }

        Ok(withdrawals)
    }
}

#[cfg(test)]
mod tests {
    use dashcore::{
        hashes::hex::{FromHex, ToHex},
        BlockHash,
    };
    use dpp::{contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture};
    use drive::{rpc::core::MockCoreRPCLike, tests::helpers::setup::setup_document};
    use serde_json::json;

    use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

    use crate::{block::BlockExecutionContext, execution::fee_pools::epoch::EpochInfo};

    mod update_withdrawal_statuses {
        use crate::block::BlockStateInfo;
        use crate::test::helpers::setup::setup_platform_with_initial_state_structure;
        use dpp::{
            data_contract::{DataContract, DriveContractExt},
            system_data_contracts::{load_system_data_contract, SystemDataContract},
        };
        use drive::tests::helpers::setup::setup_system_data_contract;

        use super::*;

        #[test]
        fn test_statuses_are_updated() {
            let mut platform = setup_platform_with_initial_state_structure(None);

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

            let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
                .expect("to load system data contract");

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
                    "status": withdrawals_contract::Status::BROADCASTED,
                    "transactionIndex": 1,
                    "transactionSignHeight": 93,
                    "transactionId": vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                }),
            );

            let document_type = data_contract
                .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)
                .expect("expected to get document type");

            setup_document(
                &platform.drive,
                &document_1,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::BROADCASTED,
                    "transactionIndex": 2,
                    "transactionSignHeight": 10,
                    "transactionId": vec![3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                }),
            );

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            platform
                .update_broadcasted_withdrawal_transaction_statuses(
                    95,
                    &BlockExecutionContext {
                        block_info: BlockStateInfo {
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
                        hpmn_count: 100,
                    },
                    Some(&transaction),
                )
                .expect("to update withdrawal statuses");

            let documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::Status::EXPIRED.into(),
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
            assert_eq!(
                documents.get(0).unwrap().id.to_vec(),
                document_2.id.to_vec()
            );

            let documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::Status::COMPLETE.into(),
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
            assert_eq!(
                documents.get(0).unwrap().id.to_vec(),
                document_1.id.to_vec()
            );
        }
    }

    mod pool_withdrawals_into_transactions {
        use dpp::data_contract::DriveContractExt;
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use drive::dpp::contracts::withdrawals_contract;
        use drive::tests::helpers::setup::setup_system_data_contract;

        use crate::block::BlockStateInfo;
        use crate::test::helpers::setup::setup_platform_with_initial_state_structure;

        use super::*;

        #[test]
        fn test_pooling() {
            let platform = setup_platform_with_initial_state_structure(None);

            let transaction = platform.drive.grove.start_transaction();

            let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
                .expect("to load system data contract");

            setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::QUEUED,
                    "transactionIndex": 1,
                }),
            );

            let document_type = data_contract
                .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)
                .expect("expected to get document type");

            setup_document(
                &platform.drive,
                &document_1,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::QUEUED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            platform
                .pool_withdrawals_into_transactions_queue(
                    &BlockExecutionContext {
                        block_info: BlockStateInfo {
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
                        hpmn_count: 100,
                    },
                    Some(&transaction),
                )
                .expect("to pool withdrawal documents into transactions");

            let updated_documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::Status::POOLED.into(),
                    Some(&transaction),
                )
                .expect("to fetch withdrawal documents");

            let tx_ids = [
                "4b74f91644215904ff1aa4122b204ba674aea74d99a17c03fbda483692bf735b",
                "897ec16cb13d802ee6acdaf55274c59f3509a4929d726bab919a962ed4a8703c",
            ];

            for document in updated_documents {
                assert_eq!(document.get_u32("$revision").expect("to get $revision"), 2);

                let tx_id: Vec<u8> = document
                    .get_bytes("transactionId")
                    .expect("to get transactionId");

                let tx_id_hex = hex::encode(tx_id);

                assert!(tx_ids.contains(&tx_id_hex.as_str()));
            }
        }
    }

    mod fetch_core_block_transactions {
        use super::*;
        use crate::test::helpers::setup::setup_platform_with_initial_state_structure;

        #[test]
        fn test_fetches_core_transactions() {
            let mut platform = setup_platform_with_initial_state_structure(None);

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

    mod build_withdrawal_transactions_from_documents {
        use crate::test::helpers::setup::setup_platform_with_initial_state_structure;
        use dpp::data_contract::DriveContractExt;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::{
            document::document_stub::DocumentStub,
            identity::state_transition::identity_credit_withdrawal_transition::Pooling,
        };
        use drive::drive::identity::withdrawals::paths::WithdrawalTransaction;
        use drive::tests::helpers::setup::setup_system_data_contract;
        use itertools::Itertools;

        use super::*;

        #[test]
        fn test_build() {
            let platform = setup_platform_with_initial_state_structure(None);

            let transaction = platform.drive.grove.start_transaction();

            let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
                .expect("to load system data contract");

            setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::POOLED,
                    "transactionIndex": 1,
                }),
            );

            let document_type = data_contract
                .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)
                .expect("expected to get document type");

            setup_document(
                &platform.drive,
                &document_1,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let documents = vec![
                DocumentStub::from_cbor(
                    &document_1.to_buffer().expect("to convert document to cbor"),
                    None,
                    None,
                )
                .expect("to create document from cbor"),
                DocumentStub::from_cbor(
                    &document_2.to_buffer().expect("to convert document to cbor"),
                    None,
                    None,
                )
                .expect("to create document from cbor"),
            ];

            let transactions = platform
                .build_withdrawal_transactions_from_documents(&documents, Some(&transaction))
                .expect("to build transactions from documents");

            assert_eq!(
                transactions
                    .values()
                    .cloned()
                    .sorted()
                    .collect::<Vec<WithdrawalTransaction>>(),
                vec![
                    (
                        vec![0, 0, 0, 0, 0, 0, 0, 0],
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 0, 0, 0, 0, 0, 0, 0, 0, 48, 230, 2, 0
                        ],
                    ),
                    (
                        vec![0, 0, 0, 0, 0, 0, 0, 1],
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 1, 0, 0, 0, 0, 0, 0, 0, 48, 230, 2, 0
                        ],
                    ),
                ]
                .into_iter()
                .sorted()
                .collect::<Vec<WithdrawalTransaction>>(),
            );
        }
    }
}
