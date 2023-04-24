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

use crate::block::BlockExecutionContext;
use crate::{
    error::{execution::ExecutionError, Error},
    platform::Platform,
    rpc::core::CoreRPCLike,
};

const WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT: u16 = 16;
const NUMBER_OF_BLOCKS_BEFORE_EXPIRED: u32 = 48;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update statuses for broadcasted withdrawals
    pub fn update_broadcasted_withdrawal_transaction_statuses(
        &self,
        last_synced_core_height: u32,
        block_execution_context: &BlockExecutionContext,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let block_info = BlockInfo {
            time_ms: block_execution_context.block_state_info.block_time_ms,
            height: block_execution_context.block_state_info.height,
            core_height: block_execution_context
                .block_state_info
                .core_chain_locked_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index)?,
        };

        let data_contract_id = &withdrawals_contract::CONTRACT_ID;

        let (_, Some(contract_fetch_info)) = self.drive.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
            None,
            true,
            Some(transaction),
        )? else {
            return Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("can't fetch withdrawal data contract"),
            ));
        };

        let core_transactions = self.fetch_core_block_transactions(
            last_synced_core_height,
            block_execution_context
                .block_state_info
                .core_chain_locked_height,
        )?;

        let broadcasted_withdrawal_documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
            Some(transaction),
        )?;

        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Collecting only documents that have been updated
        let documents_to_update: Vec<Document> = broadcasted_withdrawal_documents
            .into_iter()
            .map(|mut document| {
                let transaction_sign_height: u32 = document
                    .properties
                    .get_integer(withdrawals_contract::property_names::TRANSACTION_SIGN_HEIGHT)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't get transactionSignHeight from withdrawal document",
                        ))
                    })?;

                let transaction_id_bytes = document
                    .properties
                    .get_bytes(withdrawals_contract::property_names::TRANSACTION_ID)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't get transactionId from withdrawal document",
                        ))
                    })?;

                let transaction_index = document
                    .properties
                    .get_integer(withdrawals_contract::property_names::TRANSACTION_INDEX)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't get transactionIdex from withdrawal document",
                        ))
                    })?;

                let transaction_id = hex::encode(transaction_id_bytes);

                let block_height_difference = block_execution_context
                    .block_state_info
                    .core_chain_locked_height
                    - transaction_sign_height;

                let status;

                if core_transactions.contains(&transaction_id) {
                    status = withdrawals_contract::WithdrawalStatus::COMPLETE;
                } else if block_height_difference > NUMBER_OF_BLOCKS_BEFORE_EXPIRED {
                    status = withdrawals_contract::WithdrawalStatus::EXPIRED;
                } else {
                    return Ok(None);
                };

                document.set_u8(withdrawals_contract::property_names::STATUS, status.into());

                document.set_u64(
                    withdrawals_contract::property_names::UPDATED_AT,
                    block_info.time_ms,
                );

                document.increment_revision().map_err(Error::Protocol)?;

                if status == withdrawals_contract::WithdrawalStatus::EXPIRED {
                    self.drive.add_insert_expired_index_operation(
                        transaction_index,
                        &mut drive_operations,
                    );
                }

                Ok(Some(document))
            })
            .collect::<Result<Vec<Option<Document>>, Error>>()?
            .into_iter()
            .flatten()
            .collect();

        self.drive.add_update_multiple_documents_operations(
            &documents_to_update,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
        );

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            &block_info,
            Some(transaction),
        )?;

        Ok(())
    }

    /// Prepares a list of an unsigned withdrawal transaction bytes
    pub fn fetch_and_prepare_unsigned_withdrawal_transactions(
        &self,
        validator_set_quorum_hash: [u8; 32],
        block_execution_context: &BlockExecutionContext,
        transaction: &Transaction,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let block_info = BlockInfo {
            time_ms: block_execution_context.block_state_info.block_time_ms,
            height: block_execution_context.block_state_info.height,
            core_height: block_execution_context
                .block_state_info
                .core_chain_locked_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index)?,
        };

        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let (_, Some(contract_fetch_info)) = self.drive.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
            None,
            true,
            Some(transaction),
        )? else {
            return Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("can't fetch withdrawal data contract"),
            ));
        };

        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Get 16 latest withdrawal transactions from the queue
        let untied_withdrawal_transactions = self.drive.dequeue_withdrawal_transactions(
            WITHDRAWAL_TRANSACTIONS_QUERY_LIMIT,
            Some(transaction),
            &mut drive_operations,
        )?;

        if untied_withdrawal_transactions.is_empty() {
            return Ok(Vec::new());
        }

        // Appending request_height and quorum_hash to withdrawal transaction
        // and pass it to JS Drive for singing and broadcasting
        let (unsigned_withdrawal_transactions, documents_to_update): (Vec<_>, Vec<_>) =
            untied_withdrawal_transactions
                .into_iter()
                .map(|(_, untied_transaction_bytes)| {
                    let request_info = AssetUnlockRequestInfo {
                        request_height: block_execution_context
                            .block_state_info
                            .core_chain_locked_height,
                        quorum_hash: QuorumHash::hash(&validator_set_quorum_hash),
                    };

                    let mut unsigned_transaction_bytes = vec![];

                    request_info
                        .consensus_append_to_base_encode(
                            untied_transaction_bytes.clone(),
                            &mut unsigned_transaction_bytes,
                        )
                        .map_err(|_| {
                            Error::Execution(ExecutionError::CorruptedCodeExecution(
                                "could not add additional request info to asset unlock transaction",
                            ))
                        })?;

                    let original_transaction_id = hash::hash_to_vec(untied_transaction_bytes);
                    let update_transaction_id =
                        hash::hash_to_vec(unsigned_transaction_bytes.clone());

                    let mut document = self.drive.find_withdrawal_document_by_transaction_id(
                        &original_transaction_id,
                        Some(transaction),
                    )?;

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

                    Ok((unsigned_transaction_bytes, document))
                })
                .collect::<Result<Vec<(Vec<u8>, Document)>, Error>>()?
                .into_iter()
                .unzip();

        self.drive.add_update_multiple_documents_operations(
            &documents_to_update,
            &contract_fetch_info.contract,
            contract_fetch_info
                .contract
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "could not get document type",
                    ))
                })?,
            &mut drive_operations,
        );

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            &block_info,
            Some(transaction),
        )?;

        Ok(unsigned_withdrawal_transactions)
    }

    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions_queue(
        &self,
        block_execution_context: &BlockExecutionContext,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        let block_info = BlockInfo {
            time_ms: block_execution_context.block_state_info.block_time_ms,
            height: block_execution_context.block_state_info.height,
            core_height: block_execution_context
                .block_state_info
                .core_chain_locked_height,
            epoch: Epoch::new(block_execution_context.epoch_info.current_epoch_index)?,
        };

        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

        let (_, Some(contract_fetch_info)) = self.drive.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
            None,
            true,
            Some(transaction),
        )? else {
            return Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("can't fetch withdrawal data contract"),
            ));
        };

        let mut documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::QUEUED.into(),
            Some(transaction),
        )?;

        if documents.is_empty() {
            return Ok(());
        }

        let mut drive_operations = vec![];

        let withdrawal_transactions = self.build_withdrawal_transactions_from_documents(
            &documents,
            &mut drive_operations,
            Some(transaction),
        )?;

        for document in documents.iter_mut() {
            let Some((_, transaction_bytes)) = withdrawal_transactions.get(&document.id) else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution("transactions must contain a transaction")))
            };

            let transaction_id = hash::hash_to_vec(transaction_bytes);

            document.set_bytes(
                withdrawals_contract::property_names::TRANSACTION_ID,
                transaction_id.clone(),
            );

            document.set_u8(
                withdrawals_contract::property_names::STATUS,
                withdrawals_contract::WithdrawalStatus::POOLED as u8,
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
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
        );

        let withdrawal_transactions: Vec<WithdrawalTransactionIdAndBytes> = withdrawal_transactions
            .values()
            .into_iter()
            .cloned()
            .collect();

        self.drive.add_enqueue_withdrawal_transaction_operations(
            &withdrawal_transactions,
            &mut drive_operations,
        );

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            &block_info,
            Some(transaction),
        )?;

        Ok(())
    }

    /// Fetch Core transactions by range of Core heights
    pub fn fetch_core_block_transactions(
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

    /// Build list of Core transactions from withdrawal documents
    pub fn build_withdrawal_transactions_from_documents(
        &self,
        documents: &[Document],
        drive_operation_types: &mut Vec<DriveOperation>,
        transaction: TransactionArg,
    ) -> Result<HashMap<Identifier, WithdrawalTransactionIdAndBytes>, Error> {
        let mut withdrawals: HashMap<Identifier, WithdrawalTransactionIdAndBytes> = HashMap::new();

        let latest_withdrawal_index = self
            .drive
            .fetch_and_remove_latest_withdrawal_transaction_index_operations(
                drive_operation_types,
                transaction,
            )?;

        for (i, document) in documents.iter().enumerate() {
            let output_script_bytes = document
                .properties
                .get_bytes(withdrawals_contract::property_names::OUTPUT_SCRIPT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get outputScript from withdrawal document",
                    ))
                })?;

            let amount = document
                .properties
                .get_integer(withdrawals_contract::property_names::AMOUNT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get amount from withdrawal document",
                    ))
                })?;

            let core_fee_per_byte: u32 = document
                .properties
                .get_integer(withdrawals_contract::property_names::CORE_FEE_PER_BYTE)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get coreFeePerByte from withdrawal document",
                    ))
                })?;

            let state_transition_size = 190;

            let output_script: Script = Script(output_script_bytes.into());

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
                    fee: (state_transition_size * core_fee_per_byte * 1000),
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
                document.id,
                (transaction_index.to_be_bytes().to_vec(), transaction_buffer),
            );
        }

        Ok(withdrawals)
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

    use crate::{
        block::BlockExecutionContext, execution::fee_pools::epoch::EpochInfo,
        rpc::core::MockCoreRPCLike,
    };

    mod update_withdrawal_statuses {
        use crate::state::PlatformState;
        use crate::{block::BlockStateInfo, test::helpers::setup::TestPlatformBuilder};
        use dpp::identity::core_script::CoreScript;
        use dpp::platform_value::platform_value;
        use dpp::{
            data_contract::DataContract,
            prelude::Identifier,
            system_data_contracts::{load_system_data_contract, SystemDataContract},
        };
        use drive::tests::helpers::setup::setup_system_data_contract;

        use super::*;

        #[test]
        fn test_statuses_are_updated() {
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

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

            platform.core_rpc = mock_rpc_client;

            let transaction = platform.drive.grove.start_transaction();

            let block_execution_context = BlockExecutionContext {
                block_state_info: BlockStateInfo {
                    height: 1,
                    round: 0,
                    block_time_ms: 1,
                    previous_block_time_ms: Some(1),
                    proposer_pro_tx_hash: [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0,
                    ],
                    core_chain_locked_height: 96,
                    block_hash: [0; 32],
                    commit_hash: None,
                },
                epoch_info: EpochInfo {
                    current_epoch_index: 1,
                    previous_epoch_index: None,
                    is_epoch_change: false,
                },
                hpmn_count: 100,
                withdrawal_transactions: Default::default(),
                block_platform_state: PlatformState {
                    last_committed_block_info: None,
                    current_protocol_version_in_consensus: 0,
                    next_epoch_protocol_version: 0,
                    quorums_extended_info: Default::default(),
                    current_validator_set_quorum_hash: Default::default(),
                    validator_sets: Default::default(),
                    full_masternode_list: Default::default(),
                    hpmn_masternode_list: Default::default(),
                    initialization_information: None,
                },
            };

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

            let owner_id = Identifier::new([1u8; 32]);

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                    "transactionIndex": 1u64,
                    "transactionSignHeight": 93u64,
                    "transactionId": Identifier::new([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
                }),
                None,
            ).expect("expected withdrawal document");

            let document_type = data_contract
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
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
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::BROADCASTED as u8,
                    "transactionIndex": 2u64,
                    "transactionSignHeight": 10u64,
                    "transactionId": Identifier::new([3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
                }),
                None,
            ).expect("expected withdrawal document");

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
                    &block_execution_context,
                    &transaction,
                )
                .expect("to update withdrawal statuses");

            let documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
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
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
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

        use dpp::identity::core_script::CoreScript;
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
        use dpp::platform_value::platform_value;
        use dpp::prelude::Identifier;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use drive::dpp::contracts::withdrawals_contract;
        use drive::tests::helpers::setup::setup_system_data_contract;

        use crate::state::PlatformState;
        use crate::{block::BlockStateInfo, test::helpers::setup::TestPlatformBuilder};

        use super::*;

        #[test]
        fn test_pooling() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let transaction = platform.drive.grove.start_transaction();

            platform
                .block_execution_context
                .write()
                .unwrap()
                .replace(BlockExecutionContext {
                    block_state_info: BlockStateInfo {
                        height: 1,
                        round: 0,
                        block_time_ms: 1,
                        previous_block_time_ms: Some(1),
                        proposer_pro_tx_hash: [
                            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                            0, 0, 0, 0, 0, 0, 0, 0,
                        ],
                        core_chain_locked_height: 96,
                        block_hash: [0; 32],
                        commit_hash: None,
                    },
                    epoch_info: EpochInfo {
                        current_epoch_index: 1,
                        previous_epoch_index: None,
                        is_epoch_change: false,
                    },
                    hpmn_count: 100,
                    withdrawal_transactions: Default::default(),
                    block_platform_state: PlatformState {
                        last_committed_block_info: None,
                        current_protocol_version_in_consensus: 0,
                        next_epoch_protocol_version: 0,
                        quorums_extended_info: Default::default(),
                        current_validator_set_quorum_hash: Default::default(),
                        validator_sets: Default::default(),
                        full_masternode_list: Default::default(),
                        hpmn_masternode_list: Default::default(),
                        initialization_information: None,
                    },
                });

            let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
                .expect("to load system data contract");

            setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

            let owner_id = Identifier::new([1u8; 32]);

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                    "transactionIndex": 1u64,
                }),
                None,
            )
            .expect("expected withdrawal document");

            let document_type = data_contract
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
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
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                    "transactionIndex": 2u64,
                }),
                None,
            )
            .expect("expected withdrawal document");

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let guarded_block_execution_context = platform.block_execution_context.write().unwrap();
            let block_execution_context = guarded_block_execution_context.as_ref().unwrap();
            platform
                .pool_withdrawals_into_transactions_queue(block_execution_context, &transaction)
                .expect("to pool withdrawal documents into transactions");

            let updated_documents = platform
                .drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    Some(&transaction),
                )
                .expect("to fetch withdrawal documents");

            let tx_ids = [
                "4b74f91644215904ff1aa4122b204ba674aea74d99a17c03fbda483692bf735b",
                "897ec16cb13d802ee6acdaf55274c59f3509a4929d726bab919a962ed4a8703c",
            ];

            for document in updated_documents {
                assert_eq!(document.revision, Some(2));

                let tx_id: Vec<u8> = document
                    .properties
                    .get_bytes("transactionId")
                    .expect("to get transactionId");

                let tx_id_hex = hex::encode(tx_id);

                assert!(tx_ids.contains(&tx_id_hex.as_str()));
            }
        }
    }

    mod fetch_core_block_transactions {
        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

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

            platform.core_rpc = mock_rpc_client;

            let transactions = platform
                .fetch_core_block_transactions(1, 2)
                .expect("to fetch core transactions");

            assert_eq!(transactions.len(), 2);
            assert_eq!(transactions, ["1", "2"]);
        }
    }

    mod build_withdrawal_transactions_from_documents {
        use dpp::block::block_info::BlockInfo;

        use dpp::document::Document;
        use dpp::identity::core_script::CoreScript;
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
        use dpp::platform_value::platform_value;
        use dpp::prelude::Identifier;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use drive::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
        use drive::tests::helpers::setup::setup_system_data_contract;
        use itertools::Itertools;

        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

        #[test]
        fn test_build() {
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let transaction = platform.drive.grove.start_transaction();

            let data_contract = load_system_data_contract(SystemDataContract::Withdrawals)
                .expect("to load system data contract");

            setup_system_data_contract(&platform.drive, &data_contract, Some(&transaction));

            let owner_id = Identifier::new([1u8; 32]);

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
                    "transactionIndex": 1u64,
                }),
                None,
            )
            .expect("expected withdrawal document");

            let document_type = data_contract
                .document_type_for_name(withdrawals_contract::document_types::WITHDRAWAL)
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
                owner_id,
                platform_value!({
                    "amount": 1000u64,
                    "coreFeePerByte": 1u32,
                    "pooling": Pooling::Never as u8,
                    "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                    "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
                    "transactionIndex": 2u64,
                }),
                None,
            )
            .expect("expected withdrawal document");

            setup_document(
                &platform.drive,
                &document_2,
                &data_contract,
                document_type,
                Some(&transaction),
            );

            let documents = vec![document_1, document_2];

            let mut batch = vec![];

            let transactions = platform
                .build_withdrawal_transactions_from_documents(
                    &documents,
                    &mut batch,
                    Some(&transaction),
                )
                .expect("to build transactions from documents");

            platform
                .drive
                .apply_drive_operations(batch, true, &BlockInfo::default(), Some(&transaction))
                .expect("to apply drive op batch");

            assert_eq!(
                transactions
                    .values()
                    .cloned()
                    .sorted()
                    .collect::<Vec<WithdrawalTransactionIdAndBytes>>(),
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
                .collect::<Vec<WithdrawalTransactionIdAndBytes>>(),
            );
        }
    }
}
