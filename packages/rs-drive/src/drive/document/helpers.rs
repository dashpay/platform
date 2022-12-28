use dpp::{
    contracts::withdrawals_contract,
    data_contract::extra::{common, DriveContractExt},
    prelude::{DataContract, Document, Identifier},
    util::string_encoding::Encoding,
};
use grovedb::TransactionArg;
use serde_json::{json, Number, Value as JsonValue};

use crate::{
    drive::{
        block_info::BlockInfo,
        object_size_info::{DocumentAndContractInfo, DocumentInfo},
        Drive,
    },
    error::{drive::DriveError, Error},
    fee::op::DriveOperation,
};

impl Drive {
    /// Helper function to avoid boilerplate of calling an update
    pub fn add_update_document_data_operations<F>(
        &self,
        contract: &DataContract,
        document: &mut Document,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
        update_fn: F,
    ) -> Result<(), Error>
    where
        F: Fn(&mut Document) -> Result<&mut Document, Error>,
    {
        let document = update_fn(document)?;

        document.updated_at = Some(block_info.time_ms.try_into().map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't convert u64 block time to i64 updated_at",
            ))
        })?);
        document.increment_revision();

        let document_cbor = document.to_cbor().map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "Can't cbor withdrawal document",
            ))
        })?;

        let operations = self.add_document_for_contract_operations(
            DocumentAndContractInfo {
                document_info: DocumentInfo::DocumentRefAndSerialization((
                    &crate::drive::document::Document::from_cbor(&document_cbor, None, None)?,
                    &document_cbor,
                    None,
                )),
                contract,
                document_type: contract
                    .document_type_for_name(withdrawals_contract::types::WITHDRAWAL)?,
                owner_id: Some(document.owner_id.to_buffer()),
            },
            true,
            block_info,
            &mut None,
            transaction,
        )?;

        drive_operations.extend(operations);

        Ok(())
    }

    /// Helper function to find document by original transactionId
    /// and  update document transactionId to new one
    pub fn update_document_transaction_id(
        &self,
        original_transaction_id: &[u8],
        update_transaction_id: &[u8],
        block_info: &BlockInfo,
        drive_operations: &mut Vec<DriveOperation>,
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

        let (_, maybe_data_contract) = self.get_contract_with_fetch_info(
            data_contract_id.to_buffer(),
            Some(&block_info.epoch),
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

        let (documents, _, _) = self.query_documents(
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
            self.add_update_document_data_operations(
                &contract_fetch_info.contract,
                &mut document,
                block_info,
                transaction,
                drive_operations,
                |document: &mut Document| -> Result<&mut Document, Error> {
                    document
                        .set(
                            "transactionId",
                            JsonValue::Array(
                                update_transaction_id
                                    .iter()
                                    .map(|byte| JsonValue::Number(Number::from(*byte)))
                                    .collect(),
                            ),
                        )
                        .map_err(|_| {
                            Error::Drive(DriveError::CorruptedCodeExecution(
                                "Can't set document field: transactionId",
                            ))
                        })?;

                    Ok(document)
                },
            )?;
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

    use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

    use crate::{drive::block_info::BlockInfo, fee_pools::epochs::Epoch};

    mod update_document_transaction_id {

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
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 1,
                    "transactionId": original_transaction_id,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let block_info = BlockInfo {
                time_ms: 1,
                height: 1,
                epoch: Epoch::new(1),
            };

            let mut drive_operations = vec![];
            let mut result_operations = vec![];

            drive
                .update_document_transaction_id(
                    &original_transaction_id,
                    &updated_transaction_id,
                    &block_info,
                    &mut drive_operations,
                    Some(&transaction),
                )
                .expect("to update transactionId");

            dbg!(&drive_operations);

            drive
                .apply_batch_drive_operations(
                    None,
                    Some(&transaction),
                    drive_operations,
                    &mut result_operations,
                )
                .expect("to apply batch drive operations");

            let updated_documents = drive
                .fetch_withdrawal_documents_by_status(
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
