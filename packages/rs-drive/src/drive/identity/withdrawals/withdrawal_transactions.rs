use std::collections::HashMap;

use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    },
    consensus::Encodable,
    Script, TxOut,
};
use dpp::{
    identity::convert_credits_to_satoshi,
    prelude::{Document, Identifier},
    util::json_value::JsonValueExt,
};
use grovedb::TransactionArg;

use crate::{
    drive::Drive,
    error::{drive::DriveError, Error},
};

use super::paths::WithdrawalTransaction;

impl Drive {
    /// Build list of Core transactions from withdrawal documents
    pub fn build_withdrawal_transactions_from_documents(
        &self,
        documents: &[Document],
        transaction: TransactionArg,
    ) -> Result<HashMap<Identifier, WithdrawalTransaction>, Error> {
        let mut withdrawals: HashMap<Identifier, WithdrawalTransaction> = HashMap::new();

        let latest_withdrawal_index =
            self.fetch_latest_withdrawal_transaction_index(transaction)?;

        for (i, document) in documents.iter().enumerate() {
            let output_script = document.get_data().get_bytes("outputScript").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get outputScript from withdrawal document",
                ))
            })?;

            let amount = document.get_data().get_u64("amount").map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't get amount from withdrawal document",
                ))
            })?;

            let core_fee_per_byte =
                document.get_data().get_u64("coreFeePerByte").map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't get coreFeePerByte from withdrawal document",
                    ))
                })?;

            let state_transition_size = 190;

            let output_script: Script = Script::from(output_script);

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
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't consensus encode a withdrawal transaction",
                    ))
                })?;

            withdrawals.insert(
                document.id.clone(),
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
    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;

    use serde_json::json;

    use dpp::{
        contracts::withdrawals_contract,
        tests::fixtures::{get_withdrawal_document_fixture, get_withdrawals_data_contract_fixture},
    };

    use crate::{
        common::helpers::setup::{setup_document, setup_system_data_contract},
        drive::identity::withdrawals::paths::WithdrawalTransaction,
    };

    mod build_withdrawal_transactions_from_documents {
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;
        use itertools::Itertools;

        use super::*;

        #[test]
        fn test_build() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 1,
                }),
            );

            setup_document(&drive, &document_1, &data_contract, Some(&transaction));

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document_2, &data_contract, Some(&transaction));

            let documents = vec![document_1, document_2];

            let transactions = drive
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
                            1, 0, 0, 0, 0, 0, 0, 0, 0, 192, 206, 2, 0,
                        ],
                    ),
                    (
                        vec![0, 0, 0, 0, 0, 0, 0, 1],
                        vec![
                            1, 0, 9, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 23, 0, 1, 2, 3, 4, 5, 6, 7,
                            8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 0, 0, 0, 0,
                            1, 1, 0, 0, 0, 0, 0, 0, 0, 192, 206, 2, 0,
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
