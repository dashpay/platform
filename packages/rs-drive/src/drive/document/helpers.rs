use dpp::{
    contracts::withdrawals_contract,
    data_contract::extra::{common, DocumentType, DriveContractExt},
    prelude::{DataContract, Document, Identifier},
    util::string_encoding::Encoding,
};
use grovedb::TransactionArg;
use serde_json::{json, Number, Value as JsonValue};

use crate::{
    drive::{
        batch::{
            drive_op_batch::{
                DocumentOperation, DocumentOperationsForContractDocumentType, UpdateOperationInfo,
            },
            DocumentOperationType, DriveOperationType,
        },
        block_info::BlockInfo,
        Drive,
    },
    error::{drive::DriveError, Error},
};

impl Drive {
    /// Helper function to avoid boilerplate of calling an update
    pub fn update_document_data<'a, F>(
        &'a self,
        document: &mut Document,
        block_info: &BlockInfo,
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

        Ok(())
    }

    /// Helper function to find document by original transactionId
    /// and  update document transactionId to new one
    pub fn find_and_update_document_by_transaction_id(
        &self,
        original_transaction_id: &[u8],
        update_transaction_id: &[u8],
        block_info: &BlockInfo,
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

        let mut documents = documents
            .into_iter()
            .map(|document_cbor| {
                Document::from_cbor(document_cbor).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't create a document from cbor",
                    ))
                })
            })
            .collect::<Result<Vec<Document>, Error>>()?;

        let document =
            documents
                .get_mut(0)
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "Document was not found by transactionId",
                )))?;

        self.update_document_data(
            document,
            block_info,
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
        )
    }

    /// Add update multiple documents operations
    pub fn add_update_multiple_documents_operations<'a>(
        &self,
        documents: &'a [crate::contract::document::Document],
        data_contract: &'a DataContract,
        document_type: &'a DocumentType,
        drive_operations: &mut Vec<DriveOperationType<'a>>,
    ) {
        let operations: Vec<DocumentOperation> = documents
            .iter()
            .map(|document| {
                DocumentOperation::UpdateOperation(UpdateOperationInfo {
                    document,
                    serialized_document: None,
                    owner_id: None,
                    storage_flags: None,
                })
            })
            .collect();

        if !operations.is_empty() {
            drive_operations.push(DriveOperationType::DocumentOperation(
                DocumentOperationType::MultipleDocumentOperationsForSameContractDocumentType {
                    document_operations: DocumentOperationsForContractDocumentType {
                        operations,
                        contract: data_contract,
                        document_type,
                    },
                },
            ));
        }
    }

    /// Helper function to convert DPP documents to Drive documents
    pub fn convert_dpp_documents_to_drive_documents<'a, I>(
        &self,
        dpp_documents: I,
    ) -> Result<Vec<crate::contract::document::Document>, Error>
    where
        I: Iterator<Item = &'a Document>,
    {
        dpp_documents
            .map(|document| {
                crate::contract::document::Document::from_cbor(
                    &document.to_cbor().map_err(|_| {
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "Can't convert dpp document to cbor",
                        ))
                    })?,
                    None,
                    None,
                )
            })
            .collect::<Result<Vec<crate::contract::document::Document>, Error>>()
    }
}

// #[cfg(test)]
// mod tests {
//     use dpp::{
//         contracts::withdrawals_contract,
//         tests::fixtures::{get_withdrawal_document_fixture, get_withdrawals_data_contract_fixture},
//     };
//     use serde_json::json;

//     use crate::common::helpers::setup::{
//         setup_document, setup_drive_with_initial_state_structure, setup_system_data_contract,
//     };

//     use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

//     use crate::{drive::block_info::BlockInfo, fee_pools::epochs::Epoch};

//     mod update_document_transaction_id {

//         use super::*;

//         #[test]
//         fn test_transaction_id_updated() {
//             let drive = setup_drive_with_initial_state_structure();

//             let transaction = drive.grove.start_transaction();

//             let data_contract = get_withdrawals_data_contract_fixture(None);

//             setup_system_data_contract(&drive, &data_contract, Some(&transaction));

//             let original_transaction_id: Vec<u8> = vec![
//                 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
//                 1, 1, 1, 1,
//             ];

//             let updated_transaction_id: Vec<u8> = vec![
//                 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
//                 2, 2, 2, 2,
//             ];

//             let document = get_withdrawal_document_fixture(
//                 &data_contract,
//                 json!({
//                     "amount": 1000,
//                     "coreFeePerByte": 1,
//                     "pooling": Pooling::Never,
//                     "outputScript": (0..23).collect::<Vec<u8>>(),
//                     "status": withdrawals_contract::statuses::POOLED,
//                     "transactionIndex": 1,
//                     "transactionId": original_transaction_id,
//                 }),
//             );

//             setup_document(&drive, &document, &data_contract, Some(&transaction));

//             let block_info = BlockInfo {
//                 time_ms: 1,
//                 height: 1,
//                 epoch: Epoch::new(1),
//             };

//             let mut drive_operations = vec![];
//             let mut result_operations = vec![];

//             drive
//                 .find_and_update_document_by_transaction_id(
//                     &original_transaction_id,
//                     &updated_transaction_id,
//                     &block_info,
//                     &mut drive_operations,
//                     Some(&transaction),
//                 )
//                 .expect("to update transactionId");

//             dbg!(&drive_operations);

//             drive
//                 .apply_batch_drive_operations(
//                     None,
//                     Some(&transaction),
//                     drive_operations,
//                     &mut result_operations,
//                 )
//                 .expect("to apply batch drive operations");

//             let updated_documents = drive
//                 .fetch_withdrawal_documents_by_status(
//                     withdrawals_contract::statuses::POOLED,
//                     Some(&transaction),
//                 )
//                 .expect("to fetch withdrawal documents");

//             assert_eq!(updated_documents.len(), 1);

//             let stored_transaction_id: Vec<u8> = updated_documents
//                 .get(0)
//                 .unwrap()
//                 .data
//                 .get("transactionId")
//                 .unwrap()
//                 .as_array()
//                 .unwrap()
//                 .iter()
//                 .map(|byte| byte.as_u64().unwrap() as u8)
//                 .collect();

//             assert_eq!(stored_transaction_id, updated_transaction_id);
//         }
//     }
// }
