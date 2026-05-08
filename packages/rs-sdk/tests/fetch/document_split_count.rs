//! Mock-based integration tests for the SDK [`DocumentSplitCounts`] fetch path.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::common::{mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{documents::document_split_count_query::DocumentSplitCountQuery, Fetch},
    Sdk,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use drive_proof_verifier::DocumentSplitCounts;

#[tokio::test]
async fn test_mock_fetch_document_split_counts_returns_expected() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentSplitCountQuery::new(Arc::new(data_contract), document_type.name(), "a")
        .expect("build DocumentSplitCountQuery");

    let mut counts = BTreeMap::new();
    counts.insert(b"alice".to_vec(), 3u64);
    counts.insert(b"bob".to_vec(), 11u64);
    let expected = DocumentSplitCounts(counts);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.0.get(&b"alice".to_vec()), Some(&3u64));
    assert_eq!(retrieved.0.get(&b"bob".to_vec()), Some(&11u64));
}

#[tokio::test]
async fn test_mock_fetch_document_split_counts_empty_map() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentSplitCountQuery::new(Arc::new(data_contract), document_type.name(), "a")
        .expect("build DocumentSplitCountQuery");

    let expected = DocumentSplitCounts(BTreeMap::new());

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert!(retrieved.0.is_empty());
}

#[tokio::test]
async fn test_mock_fetch_document_split_counts_not_found() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentSplitCountQuery::new(Arc::new(data_contract), document_type.name(), "a")
        .expect("build DocumentSplitCountQuery");

    sdk.mock()
        .expect_fetch(query.clone(), None as Option<DocumentSplitCounts>)
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}
