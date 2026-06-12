//! Mock-based integration tests for the SDK count-fetch paths.
//!
//! `DocumentCount::fetch(sdk, query)` and
//! `DocumentSplitCounts::fetch(sdk, query)` both consume a
//! [`DocumentQuery`] (the same type used by
//! `Document::fetch_many`), with the count-specific shape
//! signalled via `.with_select(SelectProjection::count_star())` + optional
//! `.with_group_by(…)`. This file exercises the SDK ↔ mock-DAPI
//! seam:
//!
//! - `DocumentQuery` builds + serializes through the mock
//!   transport for every supported request shape (total, `In`-
//!   grouped, distinct-range).
//! - `Fetch for DocumentCount` and `Fetch for DocumentSplitCounts`
//!   correctly thread the query, response, and mock expectations.
//! - `MockResponse for DocumentCount` round-trips a `u64`.
//! - `MockResponse for DocumentSplitCounts` round-trips
//!   per-`(in_key, key)` entries.
//!
//! The mock transport short-circuits the wire-level verifier
//! path, so these tests pin the SDK seam — query builder →
//! `TryInto<GetDocumentsRequest>` → mock match → `MockResponse`
//! decode → `Fetch` return type. Anything that ships only
//! through the live-network path is out of scope here.
//!
//! Because `DocumentQuery` is the `Request` type for three
//! different `Fetch` impls (`Document`, `DocumentCount`,
//! `DocumentSplitCounts`), each `expect_fetch` call carries an
//! explicit turbofish so the mock recorder knows which response
//! type to register.
//!
//! Count / `group_by` need the latest (v3.1+) wire, so tests build via
//! [`count_capable_mock_sdk`], which boots a plain auto-detect mock SDK and then
//! ratchets it up via a cheap proven `fetch_current` — exactly as production
//! converges to the network's real protocol version on its first proven response.

use std::sync::Arc;

use super::common::{bootstrap_mock_sdk_to_latest, mock_data_contract, mock_document_type};
use dash_sdk::{
    platform::{documents::document_query::DocumentQuery, Fetch},
    Sdk, SdkBuilder,
};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use drive::query::conditions::{WhereClause, WhereOperator};
use drive::query::ordering::OrderClause;
use drive::query::SelectProjection;
use drive_proof_verifier::{DocumentCount, DocumentSplitCounts, SplitCountEntry};

/// Auto-detect mock SDK ratcheted to the latest protocol version (the first release
/// wiring `DRIVE_ABCI_QUERY_VERSIONS_V1`, so Count / `group_by` encode). The mock
/// short-circuits the wire verifier, so the proven `fetch_current` bootstrap is what
/// teaches the SDK the network version — no fixed pin needed.
async fn count_capable_mock_sdk() -> Sdk {
    let mut sdk = SdkBuilder::new_mock()
        .build()
        .expect("mock Sdk should be created");
    bootstrap_mock_sdk_to_latest(&mut sdk).await;
    sdk
}

#[tokio::test]
async fn test_mock_fetch_document_count_returns_expected() {
    let mut sdk = count_capable_mock_sdk().await;

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_select(SelectProjection::count_star());

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
    let mut sdk = count_capable_mock_sdk().await;

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_select(SelectProjection::count_star());

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
    let mut sdk = count_capable_mock_sdk().await;

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_select(SelectProjection::count_star());

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
/// response as per-`In`-value entries. The same `(In, prove=true)`
/// request with empty `group_by` would route to the aggregate
/// path instead — the `group_by` field is what selects the
/// per-value shape.
#[tokio::test]
async fn test_mock_fetch_document_split_counts_with_in_clause() {
    let mut sdk = count_capable_mock_sdk().await;

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
        .with_select(SelectProjection::count_star())
        .with_group_by("a");

    let expected = DocumentSplitCounts::from_verified(vec![
        SplitCountEntry {
            in_key: None,
            key: b"alpha".to_vec(),
            count: Some(7),
        },
        SplitCountEntry {
            in_key: None,
            key: b"beta".to_vec(),
            count: Some(3),
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
    let summed: u64 = retrieved.0.iter().map(|e| e.count.unwrap_or(0)).sum();
    assert_eq!(summed, 10, "alpha(7) + beta(3) = 10 docs");
}

/// `DocumentSplitCounts::fetch` with a range clause + explicit
/// `with_group_by(range_field)` exercises the SDK seam that
/// routes `(range, prove=true, group_by=[range_field])`
/// requests to the server's `RangeDistinctProof` dispatch.
#[tokio::test]
async fn test_mock_fetch_document_split_counts_with_distinct_range() {
    let mut sdk = count_capable_mock_sdk().await;

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
        .with_select(SelectProjection::count_star())
        .with_group_by("a")
        .with_limit(50);

    let expected = DocumentSplitCounts::from_verified(vec![
        SplitCountEntry {
            in_key: None,
            key: b"red".to_vec(),
            count: Some(12),
        },
        SplitCountEntry {
            in_key: None,
            key: b"green".to_vec(),
            count: Some(8),
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
/// count. The non-grouped range path returns an
/// `AggregateCountOnRange` proof shape that's NOT interchangeable
/// with the distinct path, so it matters that `group_by` drives
/// the dispatch even when the caller asks for a single `u64`.
#[tokio::test]
async fn test_mock_fetch_document_count_with_distinct_range_sums_entries() {
    let mut sdk = count_capable_mock_sdk().await;

    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    let query = DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        })
        .with_select(SelectProjection::count_star())
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

/// `DocumentSplitCounts` round-trips entries carrying both
/// `Some(_)` (verified) and `None` (caller asked but verifier was
/// silent) through the mock transport.
///
/// The mock path doesn't exercise the SDK's synthesis logic (that
/// requires a real proof + verifier), but it pins the wire/mock
/// shape: a fixture built with mixed `Some` / `None` counts must
/// survive the mock serialize/deserialize hop unchanged. Any
/// regression that flattens `None` to `Some(0)` or drops `None`
/// entries silently would fail here.
#[tokio::test]
async fn test_mock_fetch_document_split_counts_preserves_none_for_absent_in_values() {
    let mut sdk = count_capable_mock_sdk().await;

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
                Value::Text("gamma".to_string()),
            ]),
        })
        .with_select(SelectProjection::count_star())
        .with_group_by("a");

    // Mixed-shape fixture: `alpha` has a verified count, `beta` is
    // verified-zero (Some(0) from a hypothetical no-proof path or
    // a future absence-proof-equipped verifier), `gamma` is
    // verified-silent (None — the SDK synthesizes this on the
    // proof path when an In value's CountTree element doesn't
    // exist in the merk index).
    let expected = DocumentSplitCounts::from_verified(vec![
        SplitCountEntry {
            in_key: None,
            key: b"alpha".to_vec(),
            count: Some(7),
        },
        SplitCountEntry {
            in_key: None,
            key: b"beta".to_vec(),
            count: Some(0),
        },
        SplitCountEntry {
            in_key: None,
            key: b"gamma".to_vec(),
            count: None,
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
    assert_eq!(retrieved.0.len(), 3);
    assert_eq!(retrieved.0[0].count, Some(7), "alpha verified count");
    assert_eq!(retrieved.0[1].count, Some(0), "beta verified zero");
    assert_eq!(retrieved.0[2].count, None, "gamma absent from proof");
}
