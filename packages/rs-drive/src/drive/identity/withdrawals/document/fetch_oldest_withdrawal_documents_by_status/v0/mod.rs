use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::Value;
use grovedb::TransactionArg;
use indexmap::IndexMap;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    pub(super) fn fetch_oldest_withdrawal_documents_by_status_v0(
        &self,
        status: u8,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        let withdrawals_contract = self.cache.system_data_contracts.load_withdrawals();

        let document_type = withdrawals_contract.document_type_for_name(withdrawal::NAME)?;

        let mut where_clauses = BTreeMap::new();

        //todo: make this lazy loaded or const
        where_clauses.insert(
            withdrawal::properties::STATUS.to_string(),
            WhereClause {
                field: withdrawal::properties::STATUS.to_string(),
                operator: crate::query::WhereOperator::Equal,
                value: Value::U8(status),
            },
        );

        let mut order_by = IndexMap::new();

        order_by.insert(
            withdrawal::properties::UPDATED_AT.to_string(),
            OrderClause {
                field: withdrawal::properties::UPDATED_AT.to_string(),
                ascending: true,
            },
        );

        let drive_query = DriveDocumentQuery {
            contract: &withdrawals_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: where_clauses,
            },
            offset: None,
            limit: Some(limit),
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        // todo: deal with cost of this operation
        let outcome = self.query_documents(
            drive_query,
            None,
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        Ok(outcome.documents_owned())
    }

    pub(super) fn fetch_oldest_withdrawal_documents_v0(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u8, Vec<Document>>, Error> {
        let withdrawals_contract = self.cache.system_data_contracts.load_withdrawals();

        let document_type = withdrawals_contract.document_type_for_name(withdrawal::NAME)?;

        // Create a query with no where clauses to get all documents
        let where_clauses = BTreeMap::new();
        let order_by = IndexMap::new();

        let drive_query = DriveDocumentQuery {
            contract: &withdrawals_contract,
            document_type,
            internal_clauses: InternalClauses {
                primary_key_in_clause: None,
                primary_key_equal_clause: None,
                in_clause: None,
                range_clause: None,
                equal_clauses: where_clauses,
            },
            offset: None,
            limit: None, // No limit - fetch all documents
            order_by,
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        // Fetch all documents
        let outcome = self.query_documents(
            drive_query,
            None,
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;

        let all_documents = outcome.documents_owned();

        // Group documents by status
        let mut documents_by_status: BTreeMap<u8, Vec<Document>> = BTreeMap::new();

        for doc in all_documents {
            if let Ok(status) = doc
                .properties()
                .get_integer::<u8>(withdrawal::properties::STATUS)
            {
                documents_by_status.entry(status).or_default().push(doc);
            }
        }

        // Sort each status group by updatedAt ascending (oldest first)
        for documents in documents_by_status.values_mut() {
            documents.sort_by(|a, b| {
                let a_updated_at = a.updated_at().unwrap_or(0);
                let b_updated_at = b.updated_at().unwrap_or(0);
                a_updated_at.cmp(&b_updated_at)
            });
        }

        Ok(documents_by_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_QUERY_LIMIT;
    use crate::util::test_helpers::setup::{
        setup_document, setup_drive_with_initial_state_structure, setup_system_data_contract,
    };
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contracts::withdrawals_contract;
    use dpp::identifier::Identifier;
    use dpp::identity::core_script::CoreScript;
    use dpp::platform_value::platform_value;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::system_data_contracts::withdrawals_contract::v1::document_types::withdrawal;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_withdrawal_document_fixture;
    use dpp::version::PlatformVersion;
    use dpp::withdrawal::Pooling;

    #[test]
    fn test_return_list_of_documents() {
        let drive = setup_drive_with_initial_state_structure(None);

        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&drive, &data_contract, Some(&transaction));

        let documents = drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 0);

        let owner_id = Identifier::new([1u8; 32]);

        let document = get_withdrawal_document_fixture(
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
            &drive,
            &document,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let document = get_withdrawal_document_fixture(
            &data_contract,
            owner_id,
            platform_value!({
                "amount": 1000u64,
                "coreFeePerByte": 1u32,
                "pooling": Pooling::Never as u8,
                "outputScript": CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
                "status": withdrawals_contract::WithdrawalStatus::POOLED,
                "transactionIndex": 2u64,
            }),
            None,
            platform_version.protocol_version,
        )
        .expect("expected withdrawal document");

        setup_document(
            &drive,
            &document,
            &data_contract,
            document_type,
            Some(&transaction),
        );

        let documents = drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 1);

        let documents = drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED as u8,
                DEFAULT_QUERY_LIMIT,
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        assert_eq!(documents.len(), 1);
    }

    #[test]
    fn test_fetch_oldest_withdrawals_from_testnet_data() {
        use dpp::document::DocumentV0Getters;
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
        use std::fs;

        let drive = setup_drive_with_initial_state_structure(None);
        let transaction = drive.grove.start_transaction();
        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&drive, &data_contract, Some(&transaction));

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        // Read the JSON file containing withdrawal documents
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/supporting_files/withdrawals_testnet_query_issue.json"
        );
        let json_content =
            fs::read_to_string(json_path).expect("Failed to read withdrawals test data file");

        // Parse the JSON file - it contains multiple JSON objects separated by empty lines
        let documents: Vec<&str> = json_content
            .split("\n\n")
            .filter(|s| !s.trim().is_empty())
            .collect();

        for doc_json in documents {
            // Parse the document
            let doc_value: serde_json::Value =
                serde_json::from_str(doc_json).expect("Failed to parse withdrawal document JSON");

            let mut properties: Value = doc_value.clone().into();

            // Handle outputScript separately (it's base64 encoded)
            if let Some(output_script_b64) = doc_value["outputScript"].as_str() {
                use base64::{engine::general_purpose, Engine as _};
                let output_script_bytes = general_purpose::STANDARD
                    .decode(output_script_b64)
                    .expect("Failed to decode outputScript");
                let _ = properties.insert(
                    "outputScript".to_string(),
                    Value::Bytes(output_script_bytes),
                );
            }

            // Remove system fields that will be handled by get_withdrawal_document_fixture
            let _ = properties.remove("$id");
            let _ = properties.remove("$ownerId");
            let _ = properties.remove("$createdAt");
            let _ = properties.remove("$updatedAt");
            let _ = properties.remove("$revision");

            // Extract owner ID
            let owner_id_str = doc_value["$ownerId"]
                .as_str()
                .expect("$ownerId should be a string");
            let owner_id = Identifier::from_string(owner_id_str, Encoding::Base58)
                .expect("Failed to parse owner ID");

            // Extract timestamps
            let _created_at = doc_value["$createdAt"].as_u64();
            let _updated_at = doc_value["$updatedAt"].as_u64();

            // Create the document
            let document = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                properties,
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");

            setup_document(
                &drive,
                &document,
                &data_contract,
                document_type,
                Some(&transaction),
            );
        }

        // Now fetch the oldest queued withdrawal documents with a limit of 4
        let fetched_documents = drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                4, // Request 4 documents
                Some(&transaction),
                platform_version,
            )
            .expect("to fetch documents by status");

        // We should get exactly 4 documents back
        assert_eq!(
            fetched_documents.len(),
            4,
            "Expected to receive exactly 4 withdrawal documents"
        );

        // Verify they are sorted by updatedAt in ascending order (oldest first)
        for i in 1..fetched_documents.len() {
            let prev_updated_at = fetched_documents[i - 1]
                .updated_at()
                .expect("Document should have updatedAt");
            let curr_updated_at = fetched_documents[i]
                .updated_at()
                .expect("Document should have updatedAt");

            assert!(
                prev_updated_at <= curr_updated_at,
                "Documents should be sorted by updatedAt in ascending order"
            );
        }

        // Verify all returned documents have status QUEUED
        for doc in &fetched_documents {
            let status: u8 = doc
                .properties()
                .get_integer(withdrawal::properties::STATUS)
                .expect("Document should have status");
            assert_eq!(
                status,
                withdrawals_contract::WithdrawalStatus::QUEUED as u8,
                "All returned documents should have QUEUED status"
            );
        }
    }

    #[test]
    fn test_fetch_oldest_withdrawals_with_all_from_testnet_data() {
        use dpp::document::DocumentV0Getters;
        use std::fs;

        let drive = setup_drive_with_initial_state_structure(None);
        let transaction = drive.grove.start_transaction();
        let platform_version = PlatformVersion::latest();

        let data_contract =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                .expect("to load system data contract");

        setup_system_data_contract(&drive, &data_contract, Some(&transaction));

        let document_type = data_contract
            .document_type_for_name(withdrawal::NAME)
            .expect("expected to get document type");

        // Read the JSON file containing withdrawal documents
        let json_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/supporting_files/withdrawals_testnet_query_issue.json"
        );
        let json_content =
            fs::read_to_string(json_path).expect("Failed to read withdrawals test data file");

        // Parse the JSON file - it contains multiple JSON objects separated by empty lines
        let documents: Vec<&str> = json_content
            .split("\n\n")
            .filter(|s| !s.trim().is_empty())
            .collect();

        for doc_json in documents {
            // Parse the document
            let doc_value: serde_json::Value =
                serde_json::from_str(doc_json).expect("Failed to parse withdrawal document JSON");

            // Extract required fields
            let status = doc_value["status"]
                .as_u64()
                .expect("status should be a number") as u8;

            let mut properties: Value = doc_value.clone().into();

            // Handle outputScript separately (it's base64 encoded)
            if let Some(output_script_b64) = doc_value["outputScript"].as_str() {
                use base64::{engine::general_purpose, Engine as _};
                let output_script_bytes = general_purpose::STANDARD
                    .decode(output_script_b64)
                    .expect("Failed to decode outputScript");
                let _ = properties.insert(
                    "outputScript".to_string(),
                    Value::Bytes(output_script_bytes),
                );
            }

            // Remove system fields that will be handled by get_withdrawal_document_fixture
            let _ = properties.remove("$id");
            let _ = properties.remove("$ownerId");
            let _ = properties.remove("$createdAt");
            let _ = properties.remove("$updatedAt");
            let _ = properties.remove("$revision");

            // Extract owner ID
            let owner_id_str = doc_value["$ownerId"]
                .as_str()
                .expect("$ownerId should be a string");
            let owner_id = Identifier::from_string(owner_id_str, Encoding::Base58)
                .expect("Failed to parse owner ID");

            // Create the document
            let document = get_withdrawal_document_fixture(
                &data_contract,
                owner_id,
                properties,
                None,
                platform_version.protocol_version,
            )
            .expect("expected withdrawal document");

            setup_document(
                &drive,
                &document,
                &data_contract,
                document_type,
                Some(&transaction),
            );
        }

        // Test the new function that fetches all documents grouped by status
        let documents_by_status = drive
            .fetch_oldest_withdrawal_documents_v0(Some(&transaction), platform_version)
            .expect("to fetch all documents grouped by status");

        // // Check that we have documents for different statuses
        // println!("Documents grouped by status:");
        // for (status, docs) in &documents_by_status {
        //     println!("  Status {}: {} documents", status, docs.len());
        // }

        // Get QUEUED documents
        let queued_documents = documents_by_status
            .get(&(withdrawals_contract::WithdrawalStatus::QUEUED as u8))
            .expect("Should have QUEUED documents");

        // Verify we have at least 4 QUEUED documents
        assert!(
            queued_documents.len() >= 4,
            "Expected at least 4 QUEUED documents, got {}",
            queued_documents.len()
        );

        // Take the first 4 and verify they're sorted by updatedAt
        let first_four: Vec<_> = queued_documents.iter().take(4).collect();

        for i in 1..first_four.len() {
            let prev_updated_at = first_four[i - 1].updated_at().unwrap_or(0);
            let curr_updated_at = first_four[i].updated_at().unwrap_or(0);
            assert!(
                prev_updated_at <= curr_updated_at,
                "Documents should be sorted by updatedAt in ascending order"
            );
        }

        // println!(
        //     "Successfully fetched {} QUEUED documents sorted by updatedAt",
        //     queued_documents.len()
        // );
    }
}
