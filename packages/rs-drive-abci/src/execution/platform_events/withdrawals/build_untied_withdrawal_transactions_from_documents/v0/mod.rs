use dashcore_rpc::dashcore::consensus::Encodable;
use dpp::block::block_info::BlockInfo;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Setters};
use dpp::withdrawal::{WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes};

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Build list of Core transactions from withdrawal documents
    pub(super) fn build_untied_withdrawal_transactions_from_documents_v0(
        &self,
        documents: &mut Vec<Document>,
        start_index: WithdrawalTransactionIndex,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<WithdrawalTransactionIndexAndBytes>, Error> {
        documents
            .iter_mut()
            .enumerate()
            .map(|(i, document)| {
                let transaction_index = start_index + i as WithdrawalTransactionIndex;

                let withdrawal_transaction = document.try_into_asset_unlock_base_transaction_info(
                    transaction_index,
                    platform_version,
                )?;

                let mut transaction_buffer: Vec<u8> = vec![];

                withdrawal_transaction
                    .consensus_encode(&mut transaction_buffer)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't consensus encode a withdrawal transaction",
                        ))
                    })?;

                document.set_u64(withdrawal::properties::TRANSACTION_INDEX, transaction_index);

                document.set_u8(
                    withdrawal::properties::STATUS,
                    withdrawals_contract::WithdrawalStatus::POOLED as u8,
                );

                document.set_updated_at(Some(block_info.time_ms));

                document.increment_revision().map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Could not increment document revision",
                    ))
                })?;

                Ok((transaction_index, transaction_buffer))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use drive::util::test_helpers::setup::setup_document;

    mod build_withdrawal_transactions_from_documents {
        use super::*;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
        use dpp::identity::core_script::CoreScript;
        use dpp::platform_value::platform_value;
        use dpp::prelude::Identifier;
        use dpp::system_data_contracts::withdrawals_contract::WithdrawalStatus;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;
        use dpp::withdrawal::Pooling;
        use drive::util::test_helpers::setup::setup_system_data_contract;

        #[test]
        fn test_build() {
            let platform_version = PlatformVersion::latest();
            let platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let transaction = platform.drive.grove.start_transaction();

            let data_contract =
                load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
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
                    "status": WithdrawalStatus::POOLED as u8,
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
                    "status": WithdrawalStatus::POOLED as u8,
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

            let mut documents = vec![document_1, document_2];

            let block_info = BlockInfo::default_with_time(50);

            let transactions = platform
                .build_untied_withdrawal_transactions_from_documents_v0(
                    &mut documents,
                    50,
                    &block_info,
                    platform_version,
                )
                .expect("to build transactions from documents");

            assert_eq!(
                transactions,
                vec![
                    (
                        50,
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 50, 0, 0, 0, 0, 0, 0, 0, 190, 0, 0, 0
                        ],
                    ),
                    (
                        51,
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 51, 0, 0, 0, 0, 0, 0, 0, 190, 0, 0, 0
                        ],
                    ),
                ]
            );
        }
    }
}
