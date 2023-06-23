use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::document::Document;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use drive::dpp::contracts::withdrawals_contract;

use drive::drive::batch::DriveOperation;
use drive::grovedb::Transaction;

use crate::execution::types::block_execution_context;
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

const NUMBER_OF_BLOCKS_BEFORE_EXPIRED: u32 = 48;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Update statuses for broadcasted withdrawals
    pub fn update_broadcasted_withdrawal_transaction_statuses_v0(
        &self,
        last_synced_core_height: u32,
        block_execution_context: &block_execution_context::v0::BlockExecutionContext,
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

        let (_, Some(contract_fetch_info)) = self.drive.get_contract_with_fetch_info_and_fee(
            data_contract_id.to_buffer(),
            None,
            true,
            Some(transaction),
        )? else {
            return Err(Error::Execution(
                ExecutionError::CorruptedCodeExecution("can't fetch withdrawal data contract"),
            ));
        };

        let core_transactions = self.fetch_core_block_transactions_v0(
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

    use crate::execution::types::block_execution_context::v0::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfo;
    use crate::platform_types::epoch::v0::EpochInfo;
    use crate::platform_types::platform_state::v0::PlatformState;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::{
        data_contract::DataContract,
        prelude::Identifier,
        system_data_contracts::{load_system_data_contract, SystemDataContract},
    };
    use drive::tests::helpers::setup::setup_system_data_contract;

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
                bh.to_hex() == "0000000000000000000000000000000000000000000000000000000000000000"
            })
            .returning(|_| {
                Ok(json!({
                    "tx": ["0101010101010101010101010101010101010101010101010101010101010101"]
                }))
            });

        mock_rpc_client
            .expect_get_block_json()
            .withf(|bh| {
                bh.to_hex() == "1111111111111111111111111111111111111111111111111111111111111111"
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
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0,
                ],
                core_chain_locked_height: 96,
                block_hash: None,
                app_hash: None,
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
                next_validator_set_quorum_hash: None,
                validator_sets: Default::default(),
                full_masternode_list: Default::default(),
                hpmn_masternode_list: Default::default(),
                initialization_information: None,
            },
            proposer_results: None,
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
            .update_broadcasted_withdrawal_transaction_statuses_v0(
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
