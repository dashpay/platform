//! Mock-based integration tests for the SDK count-fetch paths.
//!
//! Live-devnet end-to-end coverage requires test vectors generated against a
//! running platform; for now we exercise the SDK ↔ mock-DAPI path which proves
//! that:
//!   - `DocumentCountQuery` builds + serializes through the mock transport
//!     for every supported request shape (Total, `In`, distinct-range)
//!   - `Fetch for DocumentCount` and `Fetch for DocumentSplitCounts`
//!     correctly thread the query, response, and mock expectations
//!   - `MockResponse for DocumentCount` round-trips a `u64` count
//!   - `MockResponse for DocumentSplitCounts` round-trips per-`(in_key, key)`
//!     entries (the split-count proof shape produced on `PointLookupProof` /
//!     `RangeDistinctProof` server-side paths)
//!
//! The mock transport short-circuits the wire-level verifier path, so these
//! tests don't exercise proof bytes; they pin the SDK seam — query builder →
//! `TryInto<GetDocumentsCountRequest>` → mock match → `MockResponse` decode →
//! `Fetch` return type — which is exactly the surface that earlier SDK-only
//! regressions on this PR slipped through unnoticed.

use std::sync::Arc;

use super::common::{mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{documents::document_count_query::DocumentCountQuery, Fetch},
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

/// `DocumentSplitCounts::fetch` with an `In` where-clause exercises the SDK
/// seam that routes `(In, prove=true, no-range)` requests to the
/// `PointLookupProof` server path and decodes the response as per-`In`-value
/// entries.
///
/// Pins:
/// - `DocumentCountQuery::with_where(in_clause)` builds and serializes
///   through `TryInto<GetDocumentsCountRequest>` without rejecting the
///   In operator.
/// - `Fetch for DocumentSplitCounts` correctly returns the mocked
///   per-`(in_key, key)` entries.
/// - `MockResponse for DocumentSplitCounts` round-trips `Vec<SplitCountEntry>`
///   with `in_key: None`, `key: <serialized_in_value>`, and `count` for the
///   point-lookup shape (this is the on-the-wire shape produced by
///   `verify_point_lookup_count_proof`).
#[tokio::test]
async fn test_mock_fetch_document_split_counts_with_in_clause() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentCountQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentCountQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("alpha".to_string()),
                Value::Text("beta".to_string()),
            ]),
        });

    // Mock the wire-shape entries the SDK would receive from a server-side
    // `PointLookupProof` proof verification: one entry per In branch with
    // a non-zero count, sorted lex-asc by the point-lookup builder.
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
        .expect_fetch(query.clone(), Some(expected.clone()))
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

/// `DocumentSplitCounts::fetch` with `with_distinct_counts_in_range(true)`
/// on a range query exercises the SDK seam that routes
/// `(range, prove=true, distinct=true)` requests to the
/// `RangeDistinctProof` server path and decodes the response as
/// per-distinct-value entries.
///
/// Pins:
/// - `DocumentCountQuery::with_distinct_counts_in_range(true)` + a range
///   operator builds and serializes — both knobs reach the wire request.
/// - `Fetch for DocumentSplitCounts` returns the mocked per-distinct-value
///   entries unchanged.
/// - `with_limit(Some(N))` and `with_order_by(desc)` thread through the
///   query without altering the response decode path; the limit / direction
///   are wire-level controls for the server-side walk, not client-side
///   filtering.
#[tokio::test]
async fn test_mock_fetch_document_split_counts_with_distinct_range() {
    let mut sdk = Sdk::new_mock();

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentCountQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentCountQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        })
        .with_order_by(OrderClause {
            field: "a".to_string(),
            ascending: false,
        })
        .with_distinct_counts_in_range(true)
        .with_limit(Some(50));

    // Mock the wire-shape entries from a server-side `RangeDistinctProof`
    // proof verification: per-distinct-value-in-range entries, descending
    // by terminator value because the request set `ascending: false`.
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
        .expect_fetch(query.clone(), Some(expected.clone()))
        .await
        .expect("expectation should be added");

    let retrieved = DocumentSplitCounts::fetch(&sdk, query)
        .await
        .expect("fetch should succeed")
        .expect("split counts should be present");

    assert_eq!(retrieved, expected);
    // Verify pagination knobs round-trip without disturbing the entry list.
    assert_eq!(retrieved.0.len(), 2);
    assert_eq!(retrieved.0[0].key, b"red");
    assert_eq!(retrieved.0[1].key, b"green");
}
