//! Tests for document fetch retry on stale contract deserialization error.
//!
//! When a data contract is updated on the network, the SDK may hold a cached (old) version.
//! Documents serialized with the new schema fail to deserialize with the old contract.
//! The SDK should detect this and retry once with a freshly-fetched contract.

use super::common::{mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{DocumentQuery, Fetch, FetchMany},
    Sdk,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters,
        document_type::{
            accessors::DocumentTypeV0Getters, random_document::CreateRandomDocument, DocumentType,
        },
    },
    document::{Document, DocumentV0Getters},
    prelude::DataContract,
};
use drive_proof_verifier::types::Documents;

/// Helper: create two data contracts sharing the same identity and ID, but the
/// "new" contract has an extra field.  Both contracts use the same document type name.
///
/// Returns (old_contract, new_contract, new_document_type).
fn make_old_and_new_contracts() -> (DataContract, DataContract, DocumentType) {
    use dpp::{
        data_contract::{config::DataContractConfig, DataContractFactory},
        platform_value::{platform_value, Value},
        prelude::Identifier,
        version::PlatformVersion,
    };
    use std::collections::BTreeMap;

    let platform_version = PlatformVersion::latest();
    let protocol_version = platform_version.protocol_version;
    let owner_id = Identifier::new([7u8; 32]);

    // --- "old" contract: single field ---
    let old_schema = platform_value!({
        "type": "object",
        "properties": {
            "a": {
                "type": "string",
                "maxLength": 10,
                "position": 0
            }
        },
        "additionalProperties": false,
    });

    let mut old_types: BTreeMap<String, Value> = BTreeMap::new();
    old_types.insert("document_type_name".to_string(), old_schema);

    let old_contract = DataContractFactory::new(protocol_version)
        .unwrap()
        .create(owner_id, 0, platform_value!(old_types), None, None)
        .expect("create old data contract")
        .data_contract_owned();

    // --- "new" contract: two fields (simulates a schema update) ---
    let new_schema = platform_value!({
        "type": "object",
        "properties": {
            "a": {
                "type": "string",
                "maxLength": 10,
                "position": 0
            },
            "b": {
                "type": "integer",
                "position": 1
            }
        },
        "additionalProperties": false,
    });

    let config =
        DataContractConfig::default_for_version(platform_version).expect("create a default config");

    let new_document_type = DocumentType::try_from_schema(
        old_contract.id(),
        1,
        config.version(),
        "document_type_name",
        new_schema.clone(),
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("create new document type");

    let mut new_types: BTreeMap<String, Value> = BTreeMap::new();
    new_types.insert("document_type_name".to_string(), new_schema);

    let mut new_contract = DataContractFactory::new(protocol_version)
        .unwrap()
        .create(owner_id, 0, platform_value!(new_types), None, None)
        .expect("create new data contract")
        .data_contract_owned();

    // Make the new contract share the same ID as the old one
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    new_contract.set_id(old_contract.id());

    (old_contract, new_contract, new_document_type)
}

/// Test: Document::fetch retries once when the first attempt fails with a deserialization error.
///
/// Setup:
/// 1. Create old and new versions of a data contract
/// 2. Build a DocumentQuery using the OLD contract
/// 3. Mock the document query with old contract to return a CorruptedSerialization error
/// 4. Mock DataContract::fetch to return the NEW contract
/// 5. Mock the document query with new contract to return the expected document
/// 6. Verify that Document::fetch succeeds (retry worked)
#[tokio::test]
async fn test_fetch_document_retries_on_stale_contract() {
    let mut sdk = Sdk::new_mock();

    let (old_contract, new_contract, new_doc_type) = make_old_and_new_contracts();

    let expected_document = new_doc_type
        .random_document(None, sdk.version())
        .expect("create document");
    let document_id = expected_document.id();

    // Build a query using the old contract (simulates stale cache)
    let old_query = DocumentQuery::new(old_contract.clone(), "document_type_name")
        .expect("create document query with old contract")
        .with_document_id(&document_id);

    // 1) Old query should fail with a deserialization error
    sdk.mock()
        .expect_fetch_proof_error::<Document, _>(old_query.clone())
        .await
        .expect("set error expectation");

    // 2) DataContract::fetch should return the new contract (refetch)
    sdk.mock()
        .expect_fetch(new_contract.id(), Some(new_contract.clone()))
        .await
        .expect("set contract refetch expectation");

    // 3) New query (with fresh contract) should succeed
    let new_query = old_query.clone_with_contract(std::sync::Arc::new(new_contract));
    sdk.mock()
        .expect_fetch(new_query, Some(expected_document.clone()))
        .await
        .expect("set document fetch expectation with new contract");

    // Execute: the retry logic should handle the error and succeed
    let result = Document::fetch(&sdk, old_query)
        .await
        .expect("fetch should succeed after retry");

    let fetched = result.expect("document should be returned");
    assert_eq!(fetched.id(), expected_document.id());
}

/// Test: Document::fetch_many retries once when the first attempt fails with a deserialization error.
#[tokio::test]
async fn test_fetch_many_documents_retries_on_stale_contract() {
    let mut sdk = Sdk::new_mock();

    let (old_contract, new_contract, new_doc_type) = make_old_and_new_contracts();

    let expected_document = new_doc_type
        .random_document(None, sdk.version())
        .expect("create document");

    let expected_documents =
        Documents::from([(expected_document.id(), Some(expected_document.clone()))]);

    // Build a query using the old contract (simulates stale cache)
    let old_query = DocumentQuery::new(old_contract.clone(), "document_type_name")
        .expect("create document query with old contract");

    // 1) Old query should fail with a deserialization error
    //    We use expect_fetch_proof_error with Document since DocumentQuery is the
    //    request type for both Fetch and FetchMany
    sdk.mock()
        .expect_fetch_proof_error::<Document, _>(old_query.clone())
        .await
        .expect("set error expectation");

    // 2) DataContract::fetch should return the new contract (refetch)
    sdk.mock()
        .expect_fetch(new_contract.id(), Some(new_contract.clone()))
        .await
        .expect("set contract refetch expectation");

    // 3) New query (with fresh contract) should succeed
    let new_query = old_query.clone_with_contract(std::sync::Arc::new(new_contract));
    sdk.mock()
        .expect_fetch_many(new_query, Some(expected_documents.clone()))
        .await
        .expect("set document fetch_many expectation with new contract");

    // Execute: the retry logic should handle the error and succeed
    let result = Document::fetch_many(&sdk, old_query)
        .await
        .expect("fetch_many should succeed after retry");

    assert_eq!(result.len(), 1);
    let (doc_id, doc) = result.into_iter().next().unwrap();
    assert_eq!(doc_id, expected_document.id());
    assert!(doc.is_some());
}

/// Test: When the error is NOT a deserialization error, no retry happens and the error propagates.
#[tokio::test]
async fn test_fetch_document_does_not_retry_on_other_errors() {
    let sdk = Sdk::new_mock();

    let doc_type = mock_document_type();
    let contract = mock_data_contract(Some(&doc_type));

    // Build a query but don't set any expectations — the mock will have no matching
    // response for execute, which should result in an error that is NOT a deserialization error.
    let query = DocumentQuery::new(contract, doc_type.name()).expect("create document query");

    let result = Document::fetch(&sdk, query).await;

    // Should fail — non-deserialization errors propagate without retry
    assert!(
        result.is_err(),
        "expected error when no mock expectations are set"
    );
}
