use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract::WithdrawalStatus;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Bytes32;
use dpp::prelude::Identifier;
use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::document_types::withdrawal;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

use drive::drive::batch::DriveOperation;
use drive::grovedb::Transaction;

use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
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
    pub(super) fn update_broadcasted_withdrawal_transaction_statuses_v0(
        &self,
        block_execution_context: &BlockExecutionContext,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let block_info = BlockInfo {
            time_ms: block_execution_context.block_state_info().block_time_ms(),
            height: block_execution_context.block_state_info().height(),
            core_height: block_execution_context
                .block_state_info()
                .core_chain_locked_height(),
            epoch: Epoch::new(block_execution_context.epoch_info().current_epoch_index())?,
        };

        let data_contract_id = withdrawals_contract::ID;

        let (_, Some(contract_fetch_info)) = self.drive.get_contract_with_fetch_info_and_fee(
            data_contract_id.to_buffer(),
            None,
            true,
            Some(transaction),
            platform_version,
        )?
        else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "can't fetch withdrawal data contract",
            )));
        };

        let broadcasted_withdrawal_documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
            Some(transaction),
            platform_version,
        )?;

        // Collecting only documents that have been updated
        let transactions_to_check: Vec<[u8; 32]> = broadcasted_withdrawal_documents
            .iter()
            .map(|document| {
                document
                    .properties()
                    .get_hash256_bytes(withdrawal::properties::TRANSACTION_ID)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedDriveResponse(
                            "Can't get transactionId from withdrawal document".to_string(),
                        ))
                    })
            })
            .collect::<Result<Vec<[u8; 32]>, Error>>()?;

        let core_transactions_statuses = if transactions_to_check.is_empty() {
            BTreeMap::new()
        } else {
            self.fetch_transactions_block_inclusion_status(
                block_execution_context
                    .block_state_info()
                    .core_chain_locked_height(),
                transactions_to_check,
                platform_version,
            )?
        };

        let mut drive_operations: Vec<DriveOperation> = vec![];

        // Collecting only documents that have been updated
        let documents_to_update: Vec<Document> = broadcasted_withdrawal_documents
            .into_iter()
            .map(|mut document| {
                let transaction_sign_height: u32 = document
                    .properties()
                    .get_optional_integer(withdrawal::properties::TRANSACTION_SIGN_HEIGHT)?
                    .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                        "Can't get transactionSignHeight from withdrawal document".to_string(),
                    )))?;

                let transaction_id = document
                    .properties()
                    .get_optional_hash256_bytes(withdrawal::properties::TRANSACTION_ID)?
                    .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                        "Can't get transactionId from withdrawal document".to_string(),
                    )))?;

                let transaction_index = document
                    .properties()
                    .get_optional_integer(withdrawal::properties::TRANSACTION_INDEX)?
                    .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                        "Can't get transaction index from withdrawal document".to_string(),
                    )))?;

                let current_status: WithdrawalStatus = document
                    .properties()
                    .get_optional_integer::<u8>(withdrawal::properties::STATUS)?
                    .ok_or(Error::Execution(ExecutionError::CorruptedDriveResponse(
                        "Can't get transaction index from withdrawal document".to_string(),
                    )))?
                    .try_into()
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedDriveResponse(
                            "Withdrawal status unknown".to_string(),
                        ))
                    })?;

                let block_height_difference = block_execution_context
                    .block_state_info()
                    .core_chain_locked_height()
                    - transaction_sign_height;

                let is_chain_locked =
                    *core_transactions_statuses
                        .get(&transaction_id)
                        .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "we should always have a withdrawal status",
                        )))?;

                let mut status = current_status;

                if is_chain_locked {
                    status = WithdrawalStatus::COMPLETE;
                } else if block_height_difference > NUMBER_OF_BLOCKS_BEFORE_EXPIRED {
                    status = WithdrawalStatus::EXPIRED;
                } else {
                    // todo: there could be a problem here where we always get the same withdrawals
                    //  and don't cycle them most likely when we query withdrawals
                    return Ok(None);
                }

                document.set_u8(withdrawal::properties::STATUS, status.into());

                document.set_u64(withdrawal::properties::UPDATED_AT, block_info.time_ms);

                document.increment_revision().map_err(Error::Protocol)?;

                if status == WithdrawalStatus::EXPIRED {
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
                .document_type_for_name(withdrawal::NAME)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't fetch withdrawal data contract",
                    ))
                })?,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        self.drive.apply_drive_operations(
            drive_operations,
            true,
            &block_info,
            Some(transaction),
            platform_version,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dashcore_rpc::dashcore::{hashes::hex::FromHex, BlockHash, QuorumHash};
    use dashcore_rpc::dashcore_rpc_json::GetTransactionLockedResult;
    use dpp::{
        data_contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture,
    };
    use drive::tests::helpers::setup::setup_document;
    use serde_json::json;
    use std::str::FromStr;

    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::platform_state::v0::PlatformStateV0;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TestPlatformBuilder;

    use dpp::data_contract::accessors::v0::DataContractV0Getters;

    use dpp::document::DocumentV0Getters;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;

    use crate::platform_types::platform::Platform;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::Txid;
    use dpp::system_data_contracts::withdrawals_contract::document_types::withdrawal;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::Pooling;
    use dpp::{
        prelude::Identifier,
        system_data_contracts::{load_system_data_contract, SystemDataContract},
    };
    use drive::tests::helpers::setup::setup_system_data_contract;

    #[test]
    fn test_statuses_are_updated() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let mut mock_rpc_client = MockCoreRPCLike::new();

        mock_rpc_client
            .expect_get_transactions_are_chain_locked()
            .returning(move |tx_ids: Vec<Txid>| {
                Ok(tx_ids
                    .into_iter()
                    .map(|tx_id| {
                        if tx_id.to_byte_array()
                            == [
                                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                                1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                            ]
                        {
                            GetTransactionLockedResult {
                                height: Some(93),
                                chain_lock: true,
                            }
                        } else {
                            GetTransactionLockedResult {
                                height: None,
                                chain_lock: false,
                            }
                        }
                    })
                    .collect())
            });

        mock_rpc_client
            .expect_get_block_hash()
            .withf(|height| *height == 95)
            .returning(|_| {
                Ok(BlockHash::from_str(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                )
                .unwrap())
            });

        mock_rpc_client
            .expect_get_block_hash()
            .withf(|height| *height == 96)
            .returning(|_| {
                Ok(BlockHash::from_str(
                    "1111111111111111111111111111111111111111111111111111111111111111",
                )
                .unwrap())
            });

        mock_rpc_client
            .expect_get_block_json()
            .withf(|bh| {
                hex::encode(bh)
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
                hex::encode(bh)
                    == "1111111111111111111111111111111111111111111111111111111111111111"
            })
            .returning(|_| {
                Ok(json!({
                    "tx": ["0202020202020202020202020202020202020202020202020202020202020202"]
                }))
            });

        platform.core_rpc = mock_rpc_client;

        let transaction = platform.drive.grove.start_transaction();

        let block_execution_context = BlockExecutionContextV0 {
            block_state_info: BlockStateInfoV0 {
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
            }
            .into(),
            epoch_info: EpochInfoV0 {
                current_epoch_index: 1,
                previous_epoch_index: None,
                is_epoch_change: false,
            }
            .into(),
            hpmn_count: 100,
            withdrawal_transactions: Default::default(),
            block_platform_state: PlatformStateV0 {
                last_committed_block_info: None,
                current_protocol_version_in_consensus: 0,
                next_epoch_protocol_version: 0,
                current_validator_set_quorum_hash: QuorumHash::all_zeros(),
                next_validator_set_quorum_hash: None,
                validator_sets: Default::default(),
                full_masternode_list: Default::default(),
                hpmn_masternode_list: Default::default(),
                initialization_information: None,
            }
            .into(),
            proposer_results: None,
        };

        let data_contract = load_system_data_contract(
            SystemDataContract::Withdrawals,
            platform_version.protocol_version,
        )
        .expect("to load system data contract");

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
                platform_version.protocol_version,
            ).expect("expected withdrawal document");

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
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
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        setup_document(
            &platform.drive,
            &document_2,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        platform
            .update_broadcasted_withdrawal_transaction_statuses_v0(
                &block_execution_context.into(),
                &transaction,
                platform_version,
            )
            .expect("to update withdrawal statuses");

        let documents = platform
            .drive
            .fetch_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents.get(0).unwrap().id().to_vec(),
            document_2.id().to_vec()
        );

        let documents = platform
            .drive
            .fetch_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 1);
        assert_eq!(
            documents.get(0).unwrap().id().to_vec(),
            document_1.id().to_vec()
        );
    }
}
