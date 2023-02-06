use dpp::{
    contracts::withdrawals_contract, document::document_stub::DocumentStub,
    util::serializer,
};
use grovedb::TransactionArg;
use serde_json::json;

use crate::{
    drive::Drive,
    error::{drive::DriveError, Error},
};

impl Drive {
    /// Fetch withdrawal documents by it's status
    pub fn fetch_withdrawal_documents_by_status(
        &self,
        status: u8,
        transaction: TransactionArg,
    ) -> Result<Vec<DocumentStub>, Error> {
        let query_value = json!({
            "where": [
                [withdrawals_contract::property_names::OWNER_ID, "==", withdrawals_contract::OWNER_ID.clone()],
                [withdrawals_contract::property_names::STATUS, "==", status],
            ],
            "orderBy": [
                [withdrawals_contract::property_names::CREATE_AT, "desc"],
            ]
        });

        let query_cbor = serializer::value_to_cbor(query_value, None)?;

        let (documents, _, _) = self.query_raw_documents_using_cbor_encoded_query_with_cost(
            &query_cbor,
            withdrawals_contract::CONTRACT_ID.clone().to_buffer(),
            withdrawals_contract::types::WITHDRAWAL,
            None,
            transaction,
        )?;

        let documents = documents
            .into_iter()
            .map(|document_cbor| {
                DocumentStub::from_cbor(&document_cbor, None, None).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't create a document from cbor",
                    ))
                })
            })
            .collect::<Result<Vec<DocumentStub>, Error>>()?;

        Ok(documents)
    }

    /// Find one document by it's transactionId field
    pub fn find_document_by_transaction_id(
        &self,
        original_transaction_id: &[u8],
        transaction: TransactionArg,
    ) -> Result<DocumentStub, Error> {
        let data_contract_id = withdrawals_contract::CONTRACT_ID.clone();

        let query_value = json!({
            "where": [
                [withdrawals_contract::property_names::TRANSACTION_ID, "==", original_transaction_id],
                [withdrawals_contract::property_names::STATUS, "==", withdrawals_contract::Status::POOLED],
            ],
        });

        let query_cbor = serializer::value_to_cbor(query_value, None)?;

        let (documents, _, _) = self.query_raw_documents_using_cbor_encoded_query_with_cost(
            &query_cbor,
            data_contract_id.to_buffer(),
            withdrawals_contract::types::WITHDRAWAL,
            None,
            transaction,
        )?;

        let documents = documents
            .into_iter()
            .map(|document_cbor| {
                DocumentStub::from_cbor(&document_cbor, None, None).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't create a document from cbor",
                    ))
                })
            })
            .collect::<Result<Vec<DocumentStub>, Error>>()?;

        let document = documents
            .get(0)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "Document was not found by transactionId",
            )))?
            .clone();

        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use dpp::contracts::withdrawals_contract;
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::tests::fixtures::get_withdrawals_data_contract_fixture;
    use serde_json::json;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::tests::helpers::setup::{setup_document, setup_system_data_contract};

    mod fetch_withdrawal_documents_by_status {

        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

        use super::*;

        #[test]
        fn test_return_list_of_documents() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract =
                get_withdrawals_data_contract_fixture(Some(withdrawals_contract::OWNER_ID.clone()));

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::Status::QUEUED.into(),
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 0);

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::QUEUED,
                    "transactionIndex": 1,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::Status::QUEUED.into(),
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::Status::POOLED.into(),
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);
        }
    }

    mod find_document_by_transaction_id {
        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

        use super::*;

        #[test]
        fn test_find_document_by_transaction_id() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let document = get_withdrawal_document_fixture(
                &data_contract,
                json!({
                    "amount": 1000,
                    "coreFeePerByte": 1,
                    "pooling": Pooling::Never,
                    "outputScript": (0..23).collect::<Vec<u8>>(),
                    "status": withdrawals_contract::Status::POOLED,
                    "transactionIndex": 1,
                    "transactionId": (0..32).collect::<Vec<u8>>(),
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let found_document = drive
                .find_document_by_transaction_id(&(0..32).collect::<Vec<u8>>(), Some(&transaction))
                .expect("to find document by it's transaction id");

            assert_eq!(found_document.id.to_vec(), document.id.to_vec());
        }
    }
}
