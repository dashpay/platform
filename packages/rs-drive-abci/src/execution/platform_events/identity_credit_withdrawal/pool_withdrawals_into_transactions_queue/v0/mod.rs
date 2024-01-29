use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::version::PlatformVersion;

use drive::dpp::util::hash;
use drive::drive::identity::withdrawals::WithdrawalTransactionIndexAndBytes;
use drive::grovedb::Transaction;

use dpp::system_data_contracts::withdrawals_contract;
use dpp::system_data_contracts::withdrawals_contract::document_types::withdrawal;

use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::{
    BlockStateInfoV0Getters, BlockStateInfoV0Methods,
};
use crate::platform_types::epoch_info::v0::EpochInfoV0Getters;
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
    pub(super) fn pool_withdrawals_into_transactions_queue_v0(
        &self,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // TODO: Use drive.cache.system_data_contracts.withdrawals

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

        let mut documents = self.drive.fetch_withdrawal_documents_by_status(
            withdrawals_contract::WithdrawalStatus::QUEUED.into(),
            Some(transaction),
            platform_version,
        )?;

        if documents.is_empty() {
            return Ok(());
        }

        // TODO: we need to pass index to start with and store it
        //  otherwise we this logic shared between two functions
        let untied_withdrawal_transactions = self
            .build_untied_withdrawal_transactions_from_documents(
                &documents,
                Some(transaction),
                platform_version,
            )?;

        let mut last_transaction_index = None;
        for document in documents.iter_mut() {
            let Some((transaction_index, _)) = untied_withdrawal_transactions.get(&document.id())
            else {
                return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "transactions must contain a transaction",
                )));
            };

            document.set_u64(
                withdrawal::properties::TRANSACTION_INDEX,
                *transaction_index,
            );

            // TODO: Just use number of txs
            last_transaction_index = Some(*transaction_index);

            document.set_u8(
                withdrawal::properties::STATUS,
                withdrawals_contract::WithdrawalStatus::POOLED as u8,
            );

            document.set_i64(
                withdrawal::properties::UPDATED_AT,
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

        let withdrawal_transactions: Vec<WithdrawalTransactionIndexAndBytes> =
            untied_withdrawal_transactions.into_values().collect();

        let mut drive_operations = Vec::new();

        // TODO: It's better consume transactions
        self.drive.add_enqueue_withdrawal_transaction_operations(
            &withdrawal_transactions,
            &mut drive_operations,
        );

        self.drive.add_update_multiple_documents_operations(
            &documents,
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

        if let Some(index) = last_transaction_index {
            self.drive
                .add_update_withdrawal_index_counter_operation(index + 1, &mut drive_operations);
        }

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
    use super::*;

    use dpp::data_contracts::SystemDataContract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::withdrawal::Pooling;
    use drive::tests::helpers::setup::{setup_document, setup_system_data_contract};

    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
    use dpp::platform_value::platform_value;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_pooling() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let transaction = platform.drive.grove.start_transaction();

        let block_info = BlockInfo {
            time_ms: 1,
            height: 1,
            core_height: 96,
            epoch: Epoch::default(),
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
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "transactionIndex": 1u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

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
                "status": withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "transactionIndex": 2u64,
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
            .pool_withdrawals_into_transactions_queue_v0(
                &block_info,
                &transaction,
                platform_version,
            )
            .expect("to pool withdrawal documents into transactions");

        let updated_documents = platform
            .drive
            .fetch_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch withdrawal documents");

        for (i, document) in updated_documents.into_iter().enumerate() {
            assert_eq!(document.revision(), Some(2));

            let tx_index = document
                .properties()
                .get_u64(withdrawal::properties::TRANSACTION_INDEX)
                .expect("to get transactionIndex");

            assert_eq!(tx_index, i as u64);
        }
    }
}
