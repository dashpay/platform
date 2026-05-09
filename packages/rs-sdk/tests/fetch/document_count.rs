//! Mock-based integration tests for the SDK [`DocumentCount`] fetch path.
//!
//! Live-devnet end-to-end coverage requires test vectors generated against a
//! running platform; for now we exercise the SDK ↔ mock-DAPI path which proves
//! that:
//!   - `DocumentCountQuery` builds + serializes through the mock transport
//!   - `Fetch for DocumentCount` correctly threads the query, response, and
//!     mock expectations
//!   - `MockResponse for DocumentCount` round-trips a `u64` count

use std::sync::Arc;

use super::common::{mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{documents::document_count_query::DocumentCountQuery, Fetch},
    Sdk,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use drive_proof_verifier::DocumentCount;

#[tokio::test]
async fn test_mock_fetch_document_count_returns_expected() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentCountQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentCountQuery");

    let expected = DocumentCount(7);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentCount::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("count should be present");

    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.0, 7);
}

#[tokio::test]
async fn test_mock_fetch_document_count_zero() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentCountQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentCountQuery");

    let expected = DocumentCount(0);

    sdk.mock()
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentCount::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("count should be present");

    assert_eq!(retrieved, DocumentCount(0));
}

#[tokio::test]
async fn test_mock_fetch_document_count_not_found() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentCountQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentCountQuery");

    sdk.mock()
        .expect_fetch(query.clone(), None as Option<DocumentCount>)
        .await
        .expect("expectation should be added");

    let retrieved = DocumentCount::fetch(&sdk, query)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}
