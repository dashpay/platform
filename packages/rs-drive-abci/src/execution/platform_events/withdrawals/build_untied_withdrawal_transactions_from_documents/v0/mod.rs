use dpp::block::block_info::BlockInfo;
use dpp::dashcore::consensus::Encodable;
use dpp::data_contracts::withdrawals_contract;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
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
        documents: &mut [Document],
        start_index: WithdrawalTransactionIndex,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<WithdrawalTransactionIndexAndBytes>, Credits), Error> {
        documents.iter_mut().enumerate().try_fold(
            (Vec::new(), 0u64), // Start with an empty vector for transactions and 0 for total amount.
            |(mut transactions, mut total_amount), (i, document)| {
                // Calculate the transaction index.
                let transaction_index = start_index + i as WithdrawalTransactionIndex;

                // Convert the document into the withdrawal transaction information.
                let withdrawal_transaction = document.try_into_asset_unlock_base_transaction_info(
                    transaction_index,
                    platform_version,
                )?;

                // Create a buffer to hold the encoded transaction.
                let mut transaction_buffer: Vec<u8> = vec![];

                // Get the withdrawal amount from the document properties.
                let amount: u64 = document
                    .properties()
                    .get_integer(withdrawal::properties::AMOUNT)?;

                // Add the amount to the total, checking for overflow.
                total_amount = total_amount.checked_add(amount).ok_or_else(|| {
                    Error::Execution(ExecutionError::Overflow(
                        "Overflow while calculating total amount",
                    ))
                })?;

                // Consensus encode the withdrawal transaction into the buffer.
                withdrawal_transaction
                    .consensus_encode(&mut transaction_buffer)
                    .map_err(|_| {
                        Error::Execution(ExecutionError::CorruptedCodeExecution(
                            "Can't consensus encode a withdrawal transaction",
                        ))
                    })?;

                // Update the document properties.
                document.set_u64(withdrawal::properties::TRANSACTION_INDEX, transaction_index);
                document.set_u8(
                    withdrawal::properties::STATUS,
                    withdrawals_contract::WithdrawalStatus::POOLED as u8,
                );
                document.set_updated_at(Some(block_info.time_ms));

                // Increment the document revision, handle error if it fails.
                document.increment_revision().map_err(|_| {
                    Error::Execution(ExecutionError::Overflow(
                        "Overflow when adding to document revision for withdrawals",
                    ))
                })?;

                // Add the transaction index and encoded transaction to the result.
                transactions.push((transaction_index, transaction_buffer));

                // Return the updated accumulator.
                Ok((transactions, total_amount))
            },
        )
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

            let (transactions, credits) = platform
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

            assert_eq!(credits, 2000);
        }
    }
}
