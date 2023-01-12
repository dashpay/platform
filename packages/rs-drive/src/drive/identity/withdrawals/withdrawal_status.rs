use dpp::{
    contracts::withdrawals_contract,
    data_contract::extra::common,
    prelude::{Document, Identifier},
    util::string_encoding::Encoding,
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
    ) -> Result<Vec<Document>, Error> {
        let query_value = json!({
            "where": [
                ["status", "==", status],
            ],
            "orderBy": [
                ["$createdAt", "desc"],
            ]
        });

        let query_cbor = common::value_to_cbor(query_value, None);

        let (documents, _, _) = self.query_documents(
            &query_cbor,
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
            withdrawals_contract::types::WITHDRAWAL,
            None,
            transaction,
        )?;

        let documents = documents
            .into_iter()
            .map(|document_cbor| {
                Document::from_buffer(document_cbor).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't create a document from cbor",
                    ))
                })
            })
            .collect::<Result<Vec<Document>, Error>>()?;

        Ok(documents)
    }

    /// Find one document by it's transactionId field
    pub fn find_document_by_transaction_id(
        &self,
        original_transaction_id: &[u8],
        transaction: TransactionArg,
    ) -> Result<Document, Error> {
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

        let documents = documents
            .into_iter()
            .map(|document_cbor| {
                Document::from_buffer(document_cbor).map_err(|_| {
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "Can't create a document from cbor",
                    ))
                })
            })
            .collect::<Result<Vec<Document>, Error>>()?;

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

    use crate::common::helpers::setup::setup_drive_with_initial_state_structure;
    use crate::common::helpers::setup::{setup_document, setup_system_data_contract};

    mod fetch_withdrawal_documents_by_status {

        use dpp::identity::state_transition::identity_credit_withdrawal_transition::Pooling;

        use super::*;

        #[test]
        fn test_return_list_of_documents() {
            let drive = setup_drive_with_initial_state_structure();

            let transaction = drive.grove.start_transaction();

            let data_contract = get_withdrawals_data_contract_fixture(None);

            setup_system_data_contract(&drive, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::QUEUED,
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
                    "status": withdrawals_contract::statuses::QUEUED,
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
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 2,
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::QUEUED,
                    Some(&transaction),
                )
                .expect("to fetch documents by status");

            assert_eq!(documents.len(), 1);

            let documents = drive
                .fetch_withdrawal_documents_by_status(
                    withdrawals_contract::statuses::POOLED,
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
                    "status": withdrawals_contract::statuses::POOLED,
                    "transactionIndex": 1,
                    "transactionId": (0..32).collect::<Vec<u8>>(),
                }),
            );

            setup_document(&drive, &document, &data_contract, Some(&transaction));

            let found_document = drive
                .find_document_by_transaction_id(&(0..32).collect::<Vec<u8>>(), Some(&transaction))
                .expect("to find document by it's transaction id");

            assert_eq!(found_document.id, document.id);
        }
    }
}
