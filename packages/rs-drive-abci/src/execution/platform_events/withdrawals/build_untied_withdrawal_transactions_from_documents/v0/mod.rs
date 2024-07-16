use std::collections::HashMap;

use dashcore_rpc::dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::{
        qualified_asset_unlock::ASSET_UNLOCK_TX_SIZE,
        unqualified_asset_unlock::{AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo},
    },
    consensus::Encodable,
    ScriptBuf, TxOut,
};
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;

use drive::dpp::identifier::Identifier;
use drive::dpp::identity::convert_credits_to_duffs;
use drive::drive::identity::withdrawals::{
    WithdrawalTransactionIndex, WithdrawalTransactionIndexAndBytes,
};

use crate::{
    error::{execution::ExecutionError, Error},
    platform_types::platform::Platform,
    rpc::core::CoreRPCLike,
};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Build list of Core transactions from withdrawal documents
    pub(super) fn build_untied_withdrawal_transactions_from_documents_v0(
        &self,
        documents: &[Document],
        start_index: WithdrawalTransactionIndex,
    ) -> Result<HashMap<Identifier, WithdrawalTransactionIndexAndBytes>, Error> {
        let mut withdrawals: HashMap<Identifier, WithdrawalTransactionIndexAndBytes> =
            HashMap::new();

        for (i, document) in documents.iter().enumerate() {
            let output_script_bytes = document
                .properties()
                .get_bytes(withdrawal::properties::OUTPUT_SCRIPT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get outputScript from withdrawal document",
                    ))
                })?;

            let amount = document
                .properties()
                .get_integer(withdrawal::properties::AMOUNT)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get amount from withdrawal document",
                    ))
                })?;

            let core_fee_per_byte: u32 = document
                .properties()
                .get_integer(withdrawal::properties::CORE_FEE_PER_BYTE)
                .map_err(|_| {
                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "Can't get coreFeePerByte from withdrawal document",
                    ))
                })?;

            let output_script = ScriptBuf::from_bytes(output_script_bytes);

            let tx_out = TxOut {
                value: convert_credits_to_duffs(amount)?,
                script_pubkey: output_script,
            };

            let transaction_index = start_index + i as WithdrawalTransactionIndex;

            let withdrawal_transaction = AssetUnlockBaseTransactionInfo {
                version: 1,
                lock_time: 0,
                output: vec![tx_out],
                base_payload: AssetUnlockBasePayload {
                    version: 1,
                    index: transaction_index,
                    fee: ASSET_UNLOCK_TX_SIZE as u32 * core_fee_per_byte,
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

            withdrawals.insert(document.id(), (transaction_index, transaction_buffer));
        }

        Ok(withdrawals)
    }
}

#[cfg(test)]
mod tests {

    use dpp::withdrawal::Pooling;
    use dpp::{
        data_contracts::withdrawals_contract, tests::fixtures::get_withdrawal_document_fixture,
    };
    use drive::util::test_helpers::setup::setup_document;

    mod build_withdrawal_transactions_from_documents {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
        use dpp::identity::core_script::CoreScript;
        use dpp::platform_value::platform_value;
        use dpp::prelude::Identifier;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;
        use drive::drive::identity::withdrawals::WithdrawalTransactionIndexAndBytes;
        use drive::util::test_helpers::setup::setup_system_data_contract;
        use itertools::Itertools;

        use crate::test::helpers::setup::TestPlatformBuilder;

        use super::*;

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
                    "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
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
                    "status": withdrawals_contract::WithdrawalStatus::POOLED as u8,
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

            let documents = vec![document_1, document_2];

            let transactions = platform
                .build_untied_withdrawal_transactions_from_documents_v0(&documents, 50)
                .expect("to build transactions from documents");

            assert_eq!(
                transactions
                    .values()
                    .cloned()
                    .sorted()
                    .collect::<Vec<WithdrawalTransactionIndexAndBytes>>(),
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
                .into_iter()
                .sorted()
                .collect::<Vec<WithdrawalTransactionIndexAndBytes>>(),
            );
        }
    }
}
