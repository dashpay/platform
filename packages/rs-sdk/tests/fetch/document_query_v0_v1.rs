//! Unit tests for the version-aware `DocumentQuery → GetDocumentsRequest`
//! encoder added to land the v3.0-testnet decode-error fix.
//!
//! Covers:
//! - V0 wire-shape parity: encoding with a PlatformVersion whose
//!   `document_query.default_current_version = 0` ships the legacy
//!   `getDocuments` shape (CBOR-encoded `where` / `order_by` bytes,
//!   plain `uint32` limit) inside `Version::V0(...)`.
//! - V1 wire-shape parity: encoding with the latest PlatformVersion
//!   ships the SQL-shaped surface (structured WhereClause / OrderClause,
//!   `optional uint32` limit, selects / group_by / having / offset
//!   fields) inside `Version::V1(...)`.
//! - V1-only feature rejection on V0: `group_by`, `having`,
//!   `count_star()` projection — each returns `Error::Config` rather
//!   than silently emitting an invalid V0 request the server would
//!   round-trip and reject.
//! - Dispatch by SDK version: a `DocumentQuery` whose
//!   `protocol_version_override` field points at a V0 PlatformVersion
//!   round-trips through `TryFrom` as V0; default falls back to V1.
//!
//! Builder seeding semantics (auto-detect default vs. the internal
//! `with_initial_version` seed) are covered by the in-crate unit tests in
//! `dash_sdk::sdk`.

use std::sync::Arc;

use super::common::{mock_data_contract, mock_document_type};
use dapi_grpc::platform::v0::get_documents_request::Version as ReqVersion;
use dapi_grpc::platform::v0::GetDocumentsRequest;
use dash_sdk::{platform::documents::document_query::DocumentQuery, Error as SdkError, SdkBuilder};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use dpp::version::v11::PROTOCOL_VERSION_11;
use dpp::version::{PlatformVersion, INITIAL_PROTOCOL_VERSION};
use drive::query::conditions::{WhereClause, WhereOperator};
use drive::query::ordering::OrderClause;
use drive::query::SelectProjection;

/// Build a synthetic `'static PlatformVersion` whose
/// `drive_abci.query.document_query.default_current_version` is forced
/// to `0` so the encoder dispatches onto the V0 path. Every other
/// field is cloned from `PlatformVersion::latest()` so unrelated
/// subsystems still see the binary's real version layout.
fn v0_dispatch_version() -> &'static PlatformVersion {
    let mut pv = PlatformVersion::latest().clone();
    pv.drive_abci.query.document_query.default_current_version = 0;
    pv.drive_abci.query.document_query.min_version = 0;
    pv.drive_abci.query.document_query.max_version = 0;
    Box::leak(Box::new(pv))
}

fn build_basic_document_query() -> DocumentQuery {
    let document_type = mock_document_type();
    let data_contract = mock_data_contract(Some(&document_type));
    DocumentQuery::new(Arc::new(data_contract), document_type.name())
        .expect("build DocumentQuery")
        .with_where(WhereClause {
            field: "a".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("hello".to_string()),
        })
        .with_order_by(OrderClause {
            field: "a".to_string(),
            ascending: true,
        })
        .with_limit(7)
}

#[test]
fn v1_wire_shape_with_latest_platform_version() {
    let q = build_basic_document_query();
    let req: GetDocumentsRequest = q
        .try_into_request_for_version(PlatformVersion::latest())
        .expect("encode V1");

    match req.version {
        Some(ReqVersion::V1(v1)) => {
            assert_eq!(v1.document_type, "document_type_name");
            assert_eq!(v1.where_clauses.len(), 1);
            assert_eq!(v1.order_by.len(), 1);
            assert_eq!(v1.limit, Some(7));
            assert!(v1.prove);
            // The default `SelectProjection::documents()` round-trips as a
            // single-element `selects` list — the V1 wire surface keeps
            // selects as a `repeated` field for multi-projection futures.
            assert_eq!(v1.selects.len(), 1);
            assert!(v1.group_by.is_empty());
            assert!(v1.having.is_empty());
            assert_eq!(v1.offset, None);
        }
        other => panic!("expected V1 wire, got {other:?}"),
    }
}

#[test]
fn v0_wire_shape_with_forced_v0_platform_version() {
    let q = build_basic_document_query();
    let req: GetDocumentsRequest = q
        .try_into_request_for_version(v0_dispatch_version())
        .expect("encode V0");

    match req.version {
        Some(ReqVersion::V0(v0)) => {
            assert_eq!(v0.document_type, "document_type_name");
            assert_eq!(v0.limit, 7);
            assert!(v0.prove);
            // V0 ships CBOR-encoded `where` / `order_by` bytes; the
            // shape contract is "decodes back into Value::Array of
            // 3-tuples / 2-tuples". The server-side decoder consumes
            // these via `ciborium::de::from_reader` then matches on
            // `Value::Array(clauses)` — round-trip here so the test
            // catches a future regression in either direction.
            assert!(!v0.r#where.is_empty(), "V0 where bytes must be non-empty");
            let where_value: ciborium::Value =
                ciborium::de::from_reader(v0.r#where.as_slice()).expect("decode where CBOR");
            let arr = where_value.as_array().expect("where root is array");
            assert_eq!(arr.len(), 1);
            let clause = arr[0].as_array().expect("clause is array");
            assert_eq!(clause.len(), 3);
            assert_eq!(clause[0].as_text(), Some("a"));
            assert_eq!(clause[1].as_text(), Some("="));

            assert!(
                !v0.order_by.is_empty(),
                "V0 order_by bytes must be non-empty"
            );
            let order_value: ciborium::Value =
                ciborium::de::from_reader(v0.order_by.as_slice()).expect("decode order CBOR");
            let arr = order_value.as_array().expect("order_by root is array");
            assert_eq!(arr.len(), 1);
            let clause = arr[0].as_array().expect("order clause is array");
            assert_eq!(clause.len(), 2);
            assert_eq!(clause[0].as_text(), Some("a"));
            assert_eq!(clause[1].as_text(), Some("asc"));
        }
        other => panic!("expected V0 wire, got {other:?}"),
    }
}

#[test]
fn v0_rejects_count_star_projection() {
    let q = build_basic_document_query().with_select(SelectProjection::count_star());
    let err = q
        .try_into_request_for_version(v0_dispatch_version())
        .expect_err("count_star on v0 must reject");
    match err {
        SdkError::Config(msg) => assert!(
            msg.contains("v3.1+"),
            "config error should cite v3.1+ minimum, got: {msg}"
        ),
        other => panic!("expected Error::Config, got {other:?}"),
    }
}

#[test]
fn v0_rejects_group_by() {
    let q = build_basic_document_query().with_group_by("a");
    let err = q
        .try_into_request_for_version(v0_dispatch_version())
        .expect_err("group_by on v0 must reject");
    assert!(matches!(err, SdkError::Config(_)));
}

#[test]
fn v0_rejects_having() {
    use drive::query::{
        HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    };
    let q = build_basic_document_query().with_having(vec![HavingClause {
        aggregate: HavingAggregate {
            function: HavingAggregateFunction::Count,
            field: String::new(),
        },
        operator: HavingOperator::GreaterThan,
        right: HavingRightOperand::Value(Value::U64(0)),
    }]);
    let err = q
        .try_into_request_for_version(v0_dispatch_version())
        .expect_err("having on v0 must reject");
    assert!(matches!(err, SdkError::Config(_)));
}

#[test]
fn encoder_dispatches_v0_via_query_settings_without_sdk() {
    use dash_sdk::platform::{Query, QuerySettings};
    use rs_dapi_client::RequestSettings;

    // The whole point of QuerySettings: encoder is testable without
    // `Sdk::new_mock()`. Construct the context directly from a
    // PlatformVersion whose document_query is pinned to V0 dispatch
    // and assert the wire shape comes out V0.
    let v0_pv = v0_dispatch_version();
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: v0_pv,
        prove: true,
    };
    let q = build_basic_document_query();
    let req: GetDocumentsRequest = q.query(&settings).expect("encode via QuerySettings");
    assert!(
        matches!(req.version, Some(ReqVersion::V0(_))),
        "expected V0 dispatch when settings.protocol_version pins document_query to v0"
    );

    // Same query, latest PlatformVersion (V1 dispatch) — should now
    // emit V1 wire bytes through the same code path.
    let latest_settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: PlatformVersion::latest(),
        prove: true,
    };
    let q = build_basic_document_query();
    let req: GetDocumentsRequest = q.query(&latest_settings).expect("encode via QuerySettings");
    assert!(
        matches!(req.version, Some(ReqVersion::V1(_))),
        "expected V1 dispatch when settings.protocol_version is latest"
    );
}

#[test]
fn sdk_builder_default_seeds_atomic_to_floor() {
    // Auto-detect default: the atomic seeds to the effective floor,
    // max(INITIAL_PROTOCOL_VERSION, mainnet network floor), which
    // `version()` returns until the first response ratchets it upward.
    let sdk_default = SdkBuilder::new_mock().build().expect("mock sdk");
    let expected_floor = INITIAL_PROTOCOL_VERSION.max(PROTOCOL_VERSION_11);
    assert_eq!(sdk_default.version().protocol_version, expected_floor);
}

/// PROTOCOL_VERSION_11 corresponds to Dash Platform v3.0 (testnet at the
/// time of this work). Its `document_query` bounds must pin to V0 so an
/// SDK seeded at PV_11 emits V0 wire bytes that v3.0 HPMNs accept.
#[test]
fn protocol_version_for_v3_0_pins_document_query_to_v0() {
    let pv = PlatformVersion::get(11).expect("PROTOCOL_VERSION_11 exists");
    assert_eq!(
        pv.drive_abci.query.document_query.default_current_version,
        0
    );
    assert_eq!(pv.drive_abci.query.document_query.max_version, 0);
}

/// PROTOCOL_VERSION_12 corresponds to v3.1-dev. Its `document_query`
/// bounds must keep V1 semantics (max=1, default=1) — re-binding PV_11
/// to V0 must not affect PV_12.
#[test]
fn protocol_version_for_v3_1_dev_keeps_document_query_v1() {
    let pv = PlatformVersion::get(12).expect("PROTOCOL_VERSION_12 exists");
    assert_eq!(
        pv.drive_abci.query.document_query.default_current_version,
        1
    );
    assert_eq!(pv.drive_abci.query.document_query.max_version, 1);
}

/// Wallet-team end-to-end shape: a query whose `QuerySettings.protocol_version`
/// is `PROTOCOL_VERSION_11` (Dash Platform v3.0) must dispatch to the V0 encoder —
/// proving the full plumbing works without monkey-patching
/// `PlatformVersion::latest()` clones.
#[test]
fn document_query_dispatches_v0_when_sdk_initial_version_is_v3_0_pv() {
    use dash_sdk::platform::{Query, QuerySettings};
    use rs_dapi_client::RequestSettings;

    let pv_v3_0 = PlatformVersion::get(11).expect("PROTOCOL_VERSION_11 exists");
    let request_settings = RequestSettings::default();
    let settings = QuerySettings {
        request_settings: &request_settings,
        protocol_version: pv_v3_0,
        prove: true,
    };
    let q = build_basic_document_query();
    let req: GetDocumentsRequest = q
        .query(&settings)
        .expect("encode for v3.0 PV via QuerySettings");
    assert!(
        matches!(req.version, Some(ReqVersion::V0(_))),
        "expected V0 dispatch for PROTOCOL_VERSION_11"
    );
}
