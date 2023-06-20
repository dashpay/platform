use std::ops::Deref;

use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;

use drive::dpp::contracts::withdrawals_contract;

use drive::dpp::util::hash;
use drive::drive::identity::withdrawals::WithdrawalTransactionIdAndBytes;
use drive::grovedb::Transaction;

use crate::execution::types::block_execution_context;
use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions_queue_v0(
        &self,
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

        let data_contract_id = withdrawals_contract::CONTRACT_ID.deref();

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

        let mut documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::QUEUED.into(),
            Some(transaction),
        )?;

        if documents.is_empty() {
            return Ok(());
        }

        let mut drive_operations = vec![];

        let withdrawal_transactions = self.build_withdrawal_transactions_from_documents_v0(
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

        let withdrawal_transactions: Vec<WithdrawalTransactionIdAndBytes> =
            withdrawal_transactions.values().cloned().collect();

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
}

#[cfg(test)]
mod tests {

    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::{contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture};
    use drive::tests::helpers::setup::{setup_document, setup_system_data_contract};

    use crate::execution::types::block_execution_context::v0::BlockExecutionContext;
    use crate::execution::types::block_state_info::v0::BlockStateInfo;
    use crate::platform_types::epoch::v0::EpochInfo;
    use crate::platform_types::platform_state::v0::PlatformState;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;

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
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0,
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
            .pool_withdrawals_into_transactions_queue_v0(block_execution_context, &transaction)
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
