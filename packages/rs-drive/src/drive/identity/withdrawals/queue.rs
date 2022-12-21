use std::collections::HashMap;

use dashcore::{
    blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::{
        AssetUnlockBasePayload, AssetUnlockBaseTransactionInfo,
    },
    consensus::Encodable,
    Script, TxOut,
};
use dpp::{
    contracts::withdrawals_contract,
    data_contract::extra::common,
    identity::convert_credits_to_satoshi,
    prelude::{Document, Identifier},
    util::{hash, json_value::JsonValueExt, string_encoding::Encoding},
};
use grovedb::TransactionArg;
use serde_json::{json, Number, Value as JsonValue};

use crate::{
    drive::{batch::GroveDbOpBatch, Drive},
    error::{drive::DriveError, Error},
    fee_pools::epochs::Epoch,
};

use super::{
    paths::WithdrawalTransaction,
    withdrawal_status::{fetch_withdrawal_documents_by_status, update_document_data},
};

fn build_withdrawal_transactions_from_documents(
    drive: &Drive,
    documents: &[Document],
    transaction: TransactionArg,
) -> Result<HashMap<Identifier, WithdrawalTransaction>, Error> {
    let mut withdrawals: HashMap<Identifier, WithdrawalTransaction> = HashMap::new();

    let latest_withdrawal_index = drive.fetch_latest_withdrawal_transaction_index(transaction)?;

    for (i, document) in documents.iter().enumerate() {
        let output_script = document.data.get_bytes("outputScript").map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't get outputScript from withdrawal document",
            ))
        })?;

        let amount = document.data.get_u64("amount").map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't get amount from withdrawal document",
            ))
        })?;

        let core_fee_per_byte = document.data.get_u64("coreFeePerByte").map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't get coreFeePerByte from withdrawal document",
            ))
        })?;

        let state_transition_size = 184;

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

/// Helper function to find document by original transactionId
/// and  update document transactionId to new one
pub fn update_document_transaction_id(
    drive: &Drive,
    original_transaction_id: &[u8],
    update_transaction_id: &[u8],
    block_time_ms: u64,
    block_height: u64,
    current_epoch_index: u16,
    transaction: TransactionArg,
) -> Result<(), Error> {
    let data_contract_id = Identifier::from_string(
        &withdrawals_contract::system_ids().contract_id,
        Encoding::Base58,
    )
    .map_err(|_| {
        Error::Drive(DriveError::CorruptedCodeExecution(
            "Can't create withdrawals id identifier from string",
        ))
    })?;

    let (_, maybe_data_contract) = drive.get_contract_with_fetch_info(
        data_contract_id.to_buffer(),
        Some(&Epoch::new(current_epoch_index)),
        transaction,
    )?;

    let contract_fetch_info = maybe_data_contract.ok_or(Error::Drive(
        DriveError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
    ))?;

    let query_value = json!({
        "where": [
            ["transactionId", "==", original_transaction_id],
            ["status", "==", withdrawals_contract::statuses::POOLED],
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let (documents, _, _) = drive.query_documents(
        &query_cbor,
        data_contract_id.to_buffer(),
        withdrawals_contract::types::WITHDRAWAL,
        None,
        transaction,
    )?;

    let documents = documents
        .into_iter()
        .map(|document_cbor| {
            Document::from_cbor(document_cbor).map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create a document from cbor",
                ))
            })
        })
        .collect::<Result<Vec<Document>, Error>>()?;

    for mut document in documents {
        update_document_data(
            drive,
            &contract_fetch_info.contract,
            &mut document,
            block_time_ms,
            block_height,
            current_epoch_index,
            transaction,
            |document: &mut Document| -> Result<&mut Document, Error> {
                document
                    .data
                    .insert(
                        "transactionId".to_string(),
                        JsonValue::Array(
                            update_transaction_id
                                .iter()
                                .map(|byte| JsonValue::Number(Number::from(*byte)))
                                .collect(),
                        ),
                    )
                    .map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't update document field: transactionId",
                        ))
                    })?;

                document.revision += 1;

                Ok(document)
            },
        )?;
    }

    Ok(())
}

impl Drive {
    /// Pool withdrawal documents into transactions
    pub fn pool_withdrawals_into_transactions(
        &self,
        block_time_ms: u64,
        block_height: u64,
        current_epoch_index: u16,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        let (_, maybe_data_contract) = self.get_contract_with_fetch_info(
            Identifier::from_string(
                &withdrawals_contract::system_ids().contract_id,
                Encoding::Base58,
            )
            .map_err(|_| {
                Error::Drive(DriveError::CorruptedCodeExecution(
                    "Can't create withdrawals id identifier from string",
                ))
            })?
            .to_buffer(),
            Some(&Epoch::new(current_epoch_index)),
            transaction,
        )?;

        let contract_fetch_info = maybe_data_contract.ok_or(Error::Drive(
            DriveError::CorruptedCodeExecution("Can't fetch withdrawal data contract"),
        ))?;

        let mut documents = fetch_withdrawal_documents_by_status(
            self,
            withdrawals_contract::statuses::QUEUED,
            transaction,
        )?;

        let withdrawal_transactions =
            build_withdrawal_transactions_from_documents(self, &documents, transaction)?;

        for document in documents.iter_mut() {
            let transaction_id =
                hash::hash(withdrawal_transactions.get(&document.id).unwrap().1.clone());

            update_document_data(
                self,
                &contract_fetch_info.contract,
                document,
                block_time_ms,
                block_height,
                current_epoch_index,
                transaction,
                |document: &mut Document| -> Result<&mut Document, Error> {
                    document
                        .data
                        .insert(
                            "transactionId".to_string(),
                            JsonValue::Array(
                                transaction_id
                                    .clone()
                                    .into_iter()
                                    .map(|byte| JsonValue::Number(Number::from(byte)))
                                    .collect(),
                            ),
                        )
                        .map_err(|_| {
                            Error::Drive(DriveError::CorruptedCodeExecution(
                                "Can't update document field: transactionId",
                            ))
                        })?;

                    document
                        .data
                        .insert(
                            "status".to_string(),
                            JsonValue::Number(Number::from(withdrawals_contract::statuses::POOLED)),
                        )
                        .map_err(|_| {
                            Error::Drive(DriveError::CorruptedCodeExecution(
                                "Can't update document field: status",
                            ))
                        })?;

                    document.revision += 1;

                    Ok(document)
                },
            )?;
        }

        let mut batch = GroveDbOpBatch::new();

        let withdrawal_transactions = withdrawal_transactions
            .values()
            .into_iter()
            .cloned()
            .collect();

        self.add_enqueue_withdrawal_transaction_operations(&mut batch, withdrawal_transactions);

        if !batch.is_empty() {
            self.grove_apply_batch(batch, true, transaction)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use dpp::{
        contracts::withdrawals_contract,
        tests::fixtures::{get_withdrawal_document_fixture, get_withdrawals_data_contract_fixture},
    };
    use serde_json::json;

    use crate::common::helpers::setup::{
        setup_document, setup_drive_with_initial_state_structure, setup_system_data_contract,
    };

    use crate::drive::identity::withdrawals::withdrawal_status::fetch_withdrawal_documents_by_status;

    mod build_withdrawal_transactions_from_documents {

        use crate::drive::identity::withdrawals::{
            paths::WithdrawalTransaction, queue::build_withdrawal_transactions_from_documents,
        };

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
                    "pooling": 0,
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
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document_2, &data_contract, Some(&transaction));

            let documents = vec![document_1, document_2];

            let transactions = build_withdrawal_transactions_from_documents(
                &drive,
                &documents,
                Some(&transaction),
            )
            .expect("to build transactions from documents");

            assert_eq!(
                transactions
                    .values()
                    .cloned()
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
                ],
            );
        }
    }

    mod pool_withdrawals_into_transactions {

        use super::*;

        #[test]
        fn test_pooling() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let document_1 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::QUEUED,
                    "transactionIndex": 1,
                }),
            );

            setup_document(&drive, &document_1, &data_contract, Some(&transaction));

            let document_2 = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::QUEUED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document_2, &data_contract, Some(&transaction));

            drive
                .pool_withdrawals_into_transactions(1, 1, 1, Some(&transaction))
                .expect("to pool withdrawal documents into transactions");

            let updated_documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::POOLED,
                Some(&transaction),
            )
            .expect("to fetch withdrawal documents");

            let tx_ids = [
                "73050b2f1cdc267ecd9ccd10038e4c957fc108a404704e83077a593787b5f122",
                "de7889314e9dcfc6f7b142c18acc3bd1ccbee5f37d525651cdb3d5ce7fe66700",
            ];

            for document in updated_documents {
                assert_eq!(document.revision, 2);

                let tx_id: Vec<u8> = document
                    .data
                    .get("transactionId")
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|byte| byte.as_u64().unwrap() as u8)
                    .collect();

                let tx_id_hex = hex::encode(tx_id);

                assert!(tx_ids.contains(&tx_id_hex.as_str()));
            }
        }
    }

    mod update_document_transaction_id {
        use crate::drive::identity::withdrawals::queue::update_document_transaction_id;

        use super::*;

        #[test]
        fn test_transaction_id_updated() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let original_transaction_id: Vec<u8> = vec![
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1,
            ];

            let updated_transaction_id: Vec<u8> = vec![
                2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                2, 2, 2, 2,
            ];

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": 0,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 1,
                    "transactionId": original_transaction_id,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            update_document_transaction_id(
                &drive,
                &original_transaction_id,
                &updated_transaction_id,
                1,
                1,
                1,
                Some(&transaction),
            )
            .expect("to update transactionId");

            let updated_documents = fetch_withdrawal_documents_by_status(
                &drive,
                withdrawals_contract::statuses::POOLED,
                Some(&transaction),
            )
            .expect("to fetch withdrawal documents");

            assert_eq!(updated_documents.len(), 1);

            let stored_transaction_id: Vec<u8> = updated_documents
                .get(0)
                .unwrap()
                .data
                .get("transactionId")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|byte| byte.as_u64().unwrap() as u8)
                .collect();

            assert_eq!(stored_transaction_id, updated_transaction_id);
        }
    }
}
