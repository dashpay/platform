//! Mock-based integration tests for the SDK count-fetch paths
//! on top of the unified [`DocumentQuery`] surface.
//!
//! `DocumentCount::fetch(sdk, query)` and
//! `DocumentSplitCounts::fetch(sdk, query)` both consume a
//! [`DocumentQuery`] (the same type used by
//! `Document::fetch_many`), the count-specific shape signalled
//! via `.with_select(Select::Count)` + optional `.with_group_by(…)`.
//! This file exercises the SDK ↔ mock-DAPI seam:
//!
//! - `DocumentQuery` builds + serializes through the mock
//!   transport for every supported request shape (Total, `In`-
//!   grouped, distinct-range).
//! - `Fetch for DocumentCount` and `Fetch for DocumentSplitCounts`
//!   correctly thread the query, response, and mock expectations.
//! - `MockResponse for DocumentCount` round-trips a `u64`.
//! - `MockResponse for DocumentSplitCounts` round-trips
//!   per-`(in_key, key)` entries.
//!
//! The mock transport short-circuits the wire-level verifier
//! path, so these tests pin the SDK seam — query builder →
//! `TryInto<GetDocumentsRequest>` (v1) → mock match →
//! `MockResponse` decode → `Fetch` return type — which is
//! exactly the surface that earlier SDK-only regressions on
//! this PR slipped through unnoticed.
//!
//! Because `DocumentQuery` is the `Request` type for three
//! different `Fetch` impls (`Document`, `DocumentCount`,
//! `DocumentSplitCounts`), each `expect_fetch` call carries an
//! explicit turbofish so the mock recorder knows which response
//! type to register.

use std::sync::Arc;

use super::common::{mock_data_contract, mock_document_type};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select;
use dash_sdk::{
    platform::{documents::document_query::DocumentQuery, Fetch},
    Sdk,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use drive::query::conditions::{WhereClause, WhereOperator};
use drive::query::ordering::OrderClause;
use drive_proof_verifier::{DocumentCount, DocumentSplitCounts, SplitCountEntry};

#[tokio::test]
async fn test_mock_fetch_document_count_returns_expected() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_select(Select::Count);

    let expected = DocumentCount(7);

    sdk.mock()
        .expect_fetch::<DocumentCount, _>(query.clone(), Some(expected.clone()))
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
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_select(Select::Count);

    let expected = DocumentCount(0);

    sdk.mock()
        .expect_fetch::<DocumentCount, _>(query.clone(), Some(expected.clone()))
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
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_select(Select::Count);

    sdk.mock()
        .expect_fetch::<DocumentCount, _>(query.clone(), None as Option<DocumentCount>)
        .await
        .expect("expectation should be added");

    let retrieved = DocumentCount::fetch(&sdk, query)
        .await
        .expect("fetch should succeed");

    assert!(retrieved.is_none());
}

/// `DocumentSplitCounts::fetch` with an `In` where-clause +
/// explicit `with_group_by("a")` exercises the SDK seam that
/// routes `(In, prove=true, group_by=[in_field])` requests to
/// the server's `PointLookupProof` dispatch and decodes the
/// response as per-`In`-value entries.
///
/// Pre-v1 the grouping was implicit (any In implied PerInValue);
/// v1 makes it explicit so callers can ask for the aggregate
/// (empty `group_by`) or per-value entries (`group_by =
/// [in_field]`) on the same wire shape.
#[tokio::test]
async fn test_mock_fetch_document_split_counts_with_in_clause() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("alpha".to_string()),
                Value::Text("beta".to_string()),
            ]),
        })
        .with_select(Select::Count)
        .with_group_by("a");

    let expected = DocumentSplitCounts::from_verified(vec![
        SplitCountEntry {
            in_key: None,
            key: b"alpha".to_vec(),
            count: 7,
        },
        SplitCountEntry {
            in_key: None,
            key: b"beta".to_vec(),
            count: 3,
        },
    ]);

    sdk.mock()
        .expect_fetch::<DocumentSplitCounts, _>(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.0.len(), 2);
    let summed: u64 = retrieved.0.iter().map(|e| e.count).sum();
    assert_eq!(summed, 10, "alpha(7) + beta(3) = 10 docs");
}

/// `DocumentSplitCounts::fetch` with a range clause + explicit
/// `with_group_by(range_field)` exercises the SDK seam that
/// routes `(range, prove=true, group_by=[range_field])`
/// requests to the server's `RangeDistinctProof` dispatch.
///
/// Pre-v1 this was the `return_distinct_counts_in_range = true`
/// flag; v1 expresses it as explicit `group_by`. The wire
/// effect is the same.
#[tokio::test]
async fn test_mock_fetch_document_split_counts_with_distinct_range() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        })
        .with_order_by(OrderClause {
            field: "a".to_string(),
            ascending: false,
        })
        .with_select(Select::Count)
        .with_group_by("a")
        .with_limit(50);

    let expected = DocumentSplitCounts::from_verified(vec![
        SplitCountEntry {
            in_key: None,
            key: b"red".to_vec(),
            count: 12,
        },
        SplitCountEntry {
            in_key: None,
            key: b"green".to_vec(),
            count: 8,
        },
    ]);

    sdk.mock()
        .expect_fetch::<DocumentSplitCounts, _>(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.0.len(), 2);
    assert_eq!(retrieved.0[0].key, b"red");
    assert_eq!(retrieved.0[1].key, b"green");
}

/// `DocumentCount::fetch` with a range clause + explicit
/// `with_group_by(range_field)` exercises the SDK seam that
/// routes through the `RangeDistinctProof` verifier and sums
/// the verified per-key entries to produce a single aggregate
/// count. Pin against the prior regression where every range
/// query was routed through the aggregate verifier, ignoring
/// the distinct-grouping signal.
#[tokio::test]
async fn test_mock_fetch_document_count_with_distinct_range_sums_entries() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        })
        .with_select(Select::Count)
        .with_group_by("a");

    let expected = DocumentCount(20);

    sdk.mock()
        .expect_fetch::<DocumentCount, _>(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentCount::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("count should be present");

    assert_eq!(retrieved, expected);
    assert_eq!(retrieved.0, 20);
}
