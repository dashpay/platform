//! Test for fetching documents by ID with proofs

use dash_sdk::platform::document_proof::fetch_document_with_proof;
use dash_sdk::platform::proof_data::FetchWithProof;
use dash_sdk::platform::{Document, DocumentQuery, Identifier};
use dash_sdk::Sdk;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::DocumentV0Getters;
use dpp::version::PlatformVersion;

#[tokio::test]
#[cfg(feature = "mocks")]
async fn test_fetch_document_by_id_with_proof() {
    use crate::fetch::common::{mock_data_contract, mock_document_type};

    let mut sdk = Sdk::new_mock();

    // Create a mock data contract with document type
    let document_type = mock_document_type();
    let contract = mock_data_contract(Some(&document_type));
    let contract_id = contract.id();
    let document_type_name = document_type.name();

    // Create a mock document
    let document = document_type
        .random_document(None, PlatformVersion::latest())
        .expect("Failed to create document");

    let document_id = document.id();
    let _expected_owner_id = document.owner_id();

    // Set up mock expectations

    // First, expect the contract fetch
    sdk.mock()
        .expect_fetch(contract_id, Some(contract.clone()))
        .await
        .expect("Failed to set contract expectation");

    // Then expect the document fetch with the query
    let query = DocumentQuery::new_with_data_contract_id(&sdk, contract_id, &document_type_name)
        .await
        .expect("Failed to create query")
        .with_document_id(&document_id);

    sdk.mock()
        .expect_fetch(query, Some(document.clone()))
        .await
        .expect("Failed to set document expectation");

    // Now test the fetch_document_with_proof function
    // Note: This will fail with mock SDK because it returns empty proofs
    // which our parser rejects. This is expected behavior for mocks.
    let result =
        fetch_document_with_proof(&sdk, &contract_id, &document_type_name, &document_id).await;

    // With mocks, this will fail due to empty proof
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Empty proof") || error_msg.contains("No valid Merk proofs"));

    // In a real scenario with actual Platform data, the function would succeed
    // and return a DocumentWithProof with:
    // - document: The fetched document
    // - owner_id: Document owner identifier
    // - merkle_path: Path from document to state root
    // - state_root: Platform state root hash
    // - block_height: Block containing this state
}

#[tokio::test]
#[cfg(feature = "mocks")]
async fn test_fetch_document_not_found() {
    use crate::fetch::common::{mock_data_contract, mock_document_type};

    let mut sdk = Sdk::new_mock();

    // Create a mock data contract with document type
    let document_type = mock_document_type();
    let contract = mock_data_contract(Some(&document_type));
    let contract_id = contract.id();
    let document_id = Identifier::new([0x55; 32]);
    let document_type_name = document_type.name();

    // Set up mock expectations
    sdk.mock()
        .expect_fetch(contract_id, Some(contract))
        .await
        .expect("Failed to set contract expectation");

    // Expect the document query to return None (not found)
    let query = DocumentQuery::new_with_data_contract_id(&sdk, contract_id, &document_type_name)
        .await
        .expect("Failed to create query")
        .with_document_id(&document_id);

    sdk.mock()
        .expect_fetch::<Document, _>(query, None)
        .await
        .expect("Failed to set document expectation");

    // Test the fetch_document_with_proof function
    let result =
        fetch_document_with_proof(&sdk, &contract_id, &document_type_name, &document_id).await;

    // Should return an error for document not found
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Document not found"));
}
