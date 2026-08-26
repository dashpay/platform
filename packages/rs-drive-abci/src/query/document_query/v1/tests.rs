//! Tests for the v1 `getDocuments` handler — pure wire-format
//! unification of v0 documents + the (now-removed) v0-count endpoint.
//!
//! Two layers of coverage:
//! - Top-level (this module): validate-and-route routing tests + a
//!   handful of end-to-end smoke tests for the v1 wire envelope.
//! - [`ported_v0_count_tests`] (nested below): the full v0-count
//!   integration suite, ported verbatim to the v1 request shape so
//!   the count-execution surface keeps its load-bearing coverage
//!   under the new envelope.

use super::*;
use crate::query::tests::{setup_platform, store_data_contract, store_document};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    select as v1_select, Select as V1Select, Start as V1Start,
};
use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value, having_aggregate, having_clause, order_clause,
    DocumentFieldValue as ProtoDocumentFieldValue, GetDocumentsRequestV0,
    HavingAggregate as ProtoHavingAggregate, HavingClause as ProtoHavingClause,
    OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
    WhereOperator as ProtoWhereOperator,
};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::platform_value::{platform_value, Value};
use drive::query::WhereOperator;
use drive::query::{resolve_time_range_bucket_clause, validate_resolved_time_range_clause_shapes};

/// Build a `ProtoDocumentFieldValue` from a `dpp::platform_value::Value`
/// for use inside this test module only. **Subset of the SDK's
/// `value_to_proto`** — covers the primitive types these tests
/// actually construct: `Bool` / signed + unsigned integers /
/// `Float` / `Text` / `Bytes` / `Array` / `Null`. Variants the
/// SDK supports but the tests don't need (`Bytes20/32/36`,
/// `Identifier`, `U128`/`I128` → decimal text) are intentionally
/// omitted here — a test trying to use one panics so the gap is
/// loud rather than silent. Wider fidelity lives in the SDK at
/// `rs-sdk/src/platform/documents/document_query.rs::value_to_proto`.
fn pv(value: Value) -> ProtoDocumentFieldValue {
    let variant = match value {
        Value::Bool(b) => document_field_value::Variant::BoolValue(b),
        Value::I8(i) => document_field_value::Variant::Int64Value(i as i64),
        Value::I16(i) => document_field_value::Variant::Int64Value(i as i64),
        Value::I32(i) => document_field_value::Variant::Int64Value(i as i64),
        Value::I64(i) => document_field_value::Variant::Int64Value(i),
        Value::U8(u) => document_field_value::Variant::Uint64Value(u as u64),
        Value::U16(u) => document_field_value::Variant::Uint64Value(u as u64),
        Value::U32(u) => document_field_value::Variant::Uint64Value(u as u64),
        Value::U64(u) => document_field_value::Variant::Uint64Value(u),
        Value::Float(f) => document_field_value::Variant::DoubleValue(f),
        Value::Text(s) => document_field_value::Variant::Text(s),
        Value::Bytes(b) => document_field_value::Variant::BytesValue(b),
        Value::Array(items) => {
            document_field_value::Variant::List(document_field_value::ValueList {
                values: items.into_iter().map(pv).collect(),
            })
        }
        // Picking the variant means "this operand is null"; the
        // bool payload is a placeholder per the proto-side comment
        // on the `null_value` field.
        Value::Null => document_field_value::Variant::NullValue(true),
        other => panic!("pv: unsupported test-value variant {:?}", other),
    };
    ProtoDocumentFieldValue {
        variant: Some(variant),
    }
}

/// Build a proto `WhereClause` triple `(field, operator, value)`.
fn wc(field: &str, operator: ProtoWhereOperator, value: Value) -> ProtoWhereClause {
    ProtoWhereClause {
        field: field.to_string(),
        operator: operator as i32,
        value: Some(pv(value)),
    }
}

/// Build a proto `OrderClause` (field, ascending) — field-target
/// variant of the wire's `target` oneof.
fn oc(field: &str, ascending: bool) -> ProtoOrderClause {
    ProtoOrderClause {
        target: Some(order_clause::Target::Field(field.to_string())),
        ascending,
    }
}

/// Build a proto `HavingClause` with a literal-value right
/// operand `(aggregate, operator, value)`. Convenience for the
/// rejection tests — the server rejects any non-empty `having`
/// wholesale today, so the specific aggregate function / operator
/// / value here don't need to be domain-meaningful, only
/// well-formed. A literal value is the *only* right operand the wire
/// has: `HAVING` is a boolean per-group predicate, and cross-group
/// ranking rides `ORDER BY <agg> LIMIT n` instead.
fn hc(
    function: having_aggregate::Function,
    field: &str,
    operator: having_clause::Operator,
    value: Value,
) -> ProtoHavingClause {
    ProtoHavingClause {
        aggregate: Some(ProtoHavingAggregate {
            function: function as i32,
            field: field.to_string(),
        }),
        operator: operator as i32,
        right: Some(having_clause::Right::Value(pv(value))),
    }
}

/// Build the proto `selects` field for the common single-projection
/// tests. Wraps a single `Select { function, field }` in a
/// one-element vec — the wire field is `repeated Select`, the
/// `documents` / `count_star` helpers cover the bulk of test cases.
/// Tests that need the multi-projection or unknown-discriminant
/// shapes should build the vec inline.
fn select_with(function: v1_select::Function) -> Vec<V1Select> {
    vec![V1Select {
        function: function as i32,
        field: String::new(),
    }]
}

fn select_documents() -> Vec<V1Select> {
    select_with(v1_select::Function::Documents)
}

fn select_count_star() -> Vec<V1Select> {
    select_with(v1_select::Function::Count)
}

fn empty_v1_request() -> GetDocumentsRequestV1 {
    GetDocumentsRequestV1 {
        data_contract_id: vec![0u8; 32],
        document_type: "widget".to_string(),
        where_clauses: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: None,
        prove: false,
        selects: select_documents(),
        group_by: Vec::new(),
        having: Vec::new(),
        offset: None,
    }
}

fn assert_not_yet_implemented(result: Result<&'static str, QueryError>, expected_feature: &str) {
    match result {
        Err(QueryError::Query(QuerySyntaxError::Unsupported(msg))) => {
            assert!(
                msg.contains(expected_feature) && msg.contains("not yet implemented"),
                "expected message containing '{}' and 'not yet implemented', got: {}",
                expected_feature,
                msg
            );
        }
        other => panic!(
            "expected QueryError::Query(Unsupported) for '{}', got {:?}",
            expected_feature, other
        ),
    }
}

#[test]
fn reject_having_non_empty() {
    // Non-empty `having` on a **non-aggregate** select stays rejected
    // by the unversioned gate: this request carries no `selects`, so it
    // defaults to `SELECT DOCUMENTS`, and there is no aggregate for a
    // HAVING to talk about. (The aggregate selects route through the
    // versioned helper, whose v2 table serves a single grouped clause —
    // see `having_range_tests`.)
    let request = GetDocumentsRequestV1 {
        having: vec![hc(
            having_aggregate::Function::Count,
            "",
            having_clause::Operator::GreaterThan,
            Value::U64(0),
        )],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
        "HAVING clause",
    );
}

/// Unknown `Select.Function` discriminants (e.g. `42`) are malformed
/// wire input, not future capability. The handler must classify
/// them as [`QueryError::InvalidArgument`] — `not_yet_implemented`
/// carries the contract "valid request shape, caller can keep it
/// unchanged when capability lands" which is wrong for garbage
/// enum discriminants (no future protocol value would make `42`
/// meaningful for `Select.Function`).
///
/// Pins the discriminator so a future refactor that re-collapses
/// the two error classes back together (e.g. someone replaces the
/// `InvalidArgument` with `not_yet_implemented` for "consistency"
/// with the surrounding HAVING/GROUP BY rejections) fails loudly
/// rather than silently masking malformed inputs.
#[test]
fn reject_unknown_select_enum_value_as_invalid_argument() {
    let request = GetDocumentsRequestV1 {
        // Neither 0 (DOCUMENTS) nor 1 (COUNT); a discriminant
        // outside the `Select.Function` enum's defined set.
        selects: vec![V1Select {
            function: 42,
            field: String::new(),
        }],
        ..empty_v1_request()
    };
    match validate_and_route_for_tests(&request, &[], PlatformVersion::latest()) {
        Err(QueryError::InvalidArgument(msg)) => {
            assert!(
                msg.contains("42") && msg.contains("Select"),
                "expected invalid-discriminant message naming the value and the \
                 enum, got: {}",
                msg
            );
        }
        Err(QueryError::Query(QuerySyntaxError::Unsupported(msg))) => panic!(
            "expected InvalidArgument for unknown Select.Function discriminant; got \
             not_yet_implemented(\"{}\"). The two error classes carry different \
             contracts (malformed input vs. future capability) and must not be \
             collapsed.",
            msg
        ),
        other => panic!("expected InvalidArgument, got {:?}", other),
    }
}

/// `limit: Some(0)` is invalid on the v1 `optional uint32 limit`
/// field across **every** SELECT mode. The legacy ambiguity (where
/// the same wire bytes meant "use server default" in DOCUMENTS
/// mode but `InvalidLimit` in some COUNT modes) is fixed by a
/// uniform rejection at the validation boundary.
///
/// Pins the contract end-to-end across:
/// - `SELECT DOCUMENTS` (previously `unwrap_or(0)` into v0 sentinel).
/// - `SELECT COUNT, group_by=[]` (previously rejected via
///   `is_some()` but with a mode-specific message).
/// - `SELECT COUNT, group_by=[in_field]` (same).
/// - `SELECT COUNT, group_by=[range_field]` (previously
///   accepted-as-zero).
/// - `SELECT COUNT, group_by=[in_field, range_field]` (same).
///
/// All five modes must return `QuerySyntaxError::InvalidLimit`
/// with the centralized message — not five different rejection
/// reasons.
/// Non-`None` `offset` is rejected as `not_yet_implemented` on
/// every path except the ranked one. Pins the contract for the
/// non-ranked SELECT modes (DOCUMENTS / COUNT / SUM / AVG / MIN /
/// MAX); the ranked exception and the verbatim message are pinned
/// in [`ranked_tests::offset_is_still_rejected_off_the_ranked_path`]
/// and [`ranked_tests::offset_pages_through_a_ranking`].
#[test]
fn reject_offset_uniformly_across_select_modes() {
    for select_helper in [select_documents(), select_count_star()] {
        let request = GetDocumentsRequestV1 {
            selects: select_helper,
            offset: Some(10),
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
            "OFFSET pagination",
        );
    }
}

/// `selects.len() > 1` is rejected as `not_yet_implemented` —
/// multi-projection routing + response shape are deferred to a
/// follow-up. The wire stays `repeated` so the surface is stable
/// when execution lands.
#[test]
fn reject_multi_projection_selects() {
    let request = GetDocumentsRequestV1 {
        selects: vec![
            V1Select {
                function: v1_select::Function::Count as i32,
                field: String::new(),
            },
            V1Select {
                function: v1_select::Function::Sum as i32,
                field: "amount".to_string(),
            },
        ],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
        "multi-projection SELECT",
    );
}

/// `SELECT MIN(field)` / `MAX(field)` are wire-accepted but
/// rejected at routing — execution lives in a follow-up.
#[test]
fn reject_select_min_max() {
    for (function, expected_msg) in [
        (v1_select::Function::Min, "SELECT MIN"),
        (v1_select::Function::Max, "SELECT MAX"),
    ] {
        let request = GetDocumentsRequestV1 {
            selects: vec![V1Select {
                function: function as i32,
                field: "amount".to_string(),
            }],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
            expected_msg,
        );
    }
}

/// `ORDER BY <aggregate>` (wire `OrderClause.target.aggregate`) is
/// rejected at proto-decode time — drive's `OrderClause` only
/// carries a plain field name today.
#[test]
fn reject_order_by_aggregate_target() {
    let request = GetDocumentsRequestV1 {
        order_by: vec![ProtoOrderClause {
            target: Some(order_clause::Target::Aggregate(ProtoHavingAggregate {
                function: having_aggregate::Function::Count as i32,
                field: String::new(),
            })),
            ascending: false,
        }],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
        "ORDER BY on aggregate keys",
    );
}

/// `validate_and_route_for_tests` must mirror the real
/// handler's gate ordering, not just its rejection messages, so
/// a request that hits multiple gates fails on the same one in
/// tests as in production.
///
/// Real-handler order:
/// `offset → where_clauses decode → order_by decode → selects.len > 1 → select decode → validate_and_route`.
///
/// This test builds a request that is *both* multi-projection
/// AND carries an aggregate-target order_by; the order_by gate
/// must fire first (matches the real handler), not the
/// multi-projection one.
#[test]
fn validate_and_route_for_tests_matches_real_handler_gate_order() {
    let request = GetDocumentsRequestV1 {
        // Multi-projection: would trip `selects.len > 1` gate.
        selects: vec![
            V1Select {
                function: v1_select::Function::Count as i32,
                field: String::new(),
            },
            V1Select {
                function: v1_select::Function::Sum as i32,
                field: "amount".to_string(),
            },
        ],
        // ORDER BY on aggregate: trips order_by decode (earlier
        // in the sequence than `selects.len > 1`).
        order_by: vec![ProtoOrderClause {
            target: Some(order_clause::Target::Aggregate(ProtoHavingAggregate {
                function: having_aggregate::Function::Count as i32,
                field: String::new(),
            })),
            ascending: false,
        }],
        ..empty_v1_request()
    };
    // Real handler decodes order_by before checking
    // `selects.len > 1`, so the order-by-aggregate rejection
    // must surface first.
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
        "ORDER BY on aggregate keys",
    );
}

/// `value_from_proto`'s recursion-depth cap is the only
/// structural defense against deeply-nested wire payloads on the
/// v1 surface before schema validation runs. Pin the contract
/// with a depth-2 `DocumentFieldValue` so a future refactor that
/// reorders the depth check or restores the naive recursion
/// fails this test loudly rather than silently widening the
/// attack surface.
///
/// The malformed clause is delivered via a real `WhereClause`
/// because the conversion entry point on the routing path is
/// `where_clauses_from_proto`; the inner `value_from_proto_at_depth`
/// is the actual unit under test.
#[test]
fn nested_list_rejected_at_depth_two() {
    let nested_list_value = ProtoDocumentFieldValue {
        variant: Some(document_field_value::Variant::List(
            document_field_value::ValueList {
                values: vec![ProtoDocumentFieldValue {
                    variant: Some(document_field_value::Variant::List(
                        document_field_value::ValueList {
                            values: vec![ProtoDocumentFieldValue {
                                variant: Some(document_field_value::Variant::Int64Value(1)),
                            }],
                        },
                    )),
                }],
            },
        )),
    };
    let nested_clause = ProtoWhereClause {
        field: "any".to_string(),
        operator: ProtoWhereOperator::In as i32,
        value: Some(nested_list_value),
    };
    let request = GetDocumentsRequestV1 {
        where_clauses: vec![nested_clause],
        ..empty_v1_request()
    };
    match validate_and_route_for_tests(&request, &[], PlatformVersion::latest()) {
        Err(QueryError::InvalidArgument(msg)) => {
            assert!(
                msg.contains("nested DocumentFieldValue.list"),
                "expected nested-list rejection message, got: {msg}"
            );
        }
        other => panic!(
            "expected InvalidArgument for nested DocumentFieldValue.list, got {:?}",
            other
        ),
    }
}

#[test]
fn reject_limit_some_zero_uniformly_across_select_modes() {
    let in_clauses = || {
        vec![WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        }]
    };
    let range_clauses = || {
        vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        }]
    };
    let in_and_range_clauses = || {
        vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ]
    };

    // (test_label, request_builder, where_clauses)
    let cases: Vec<(&str, GetDocumentsRequestV1, Vec<WhereClause>)> = vec![
        (
            "SELECT DOCUMENTS, group_by=[]",
            GetDocumentsRequestV1 {
                selects: select_documents(),
                limit: Some(0),
                ..empty_v1_request()
            },
            Vec::new(),
        ),
        (
            "SELECT COUNT, group_by=[] (Aggregate) with In clause",
            GetDocumentsRequestV1 {
                selects: select_count_star(),
                limit: Some(0),
                ..empty_v1_request()
            },
            in_clauses(),
        ),
        (
            "SELECT COUNT, group_by=[in_field] (GroupByIn)",
            GetDocumentsRequestV1 {
                selects: select_count_star(),
                group_by: vec!["brand".to_string()],
                limit: Some(0),
                ..empty_v1_request()
            },
            in_clauses(),
        ),
        (
            "SELECT COUNT, group_by=[range_field] (GroupByRange)",
            GetDocumentsRequestV1 {
                selects: select_count_star(),
                group_by: vec!["color".to_string()],
                limit: Some(0),
                ..empty_v1_request()
            },
            range_clauses(),
        ),
        (
            "SELECT COUNT, group_by=[in_field, range_field] (GroupByCompound)",
            GetDocumentsRequestV1 {
                selects: select_count_star(),
                group_by: vec!["brand".to_string(), "color".to_string()],
                limit: Some(0),
                ..empty_v1_request()
            },
            in_and_range_clauses(),
        ),
    ];

    for (label, request, where_clauses) in cases {
        match validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()) {
            Err(QueryError::Query(QuerySyntaxError::InvalidLimit(msg))) => {
                assert!(
                    msg.contains("limit = 0") && msg.contains("v1"),
                    "[{}] expected centralized `limit = 0` rejection message, \
                     got: {}",
                    label,
                    msg
                );
            }
            other => panic!(
                "[{}] expected QuerySyntaxError::InvalidLimit for limit=Some(0); \
                 got {:?}. If this case now accepts Some(0) the v1 contract is \
                 no longer uniform — every wire-visible `Some(0)` must be \
                 rejected at the validation boundary.",
                label, other
            ),
        }
    }
}

/// GROUP BY with SELECT DOCUMENTS is structurally nonsensical
/// (GROUP BY → one row per key; DOCUMENTS → underlying rows),
/// so the rejection uses `InvalidArgument`, not
/// `not_yet_implemented`. There's no protocol version where the
/// combination becomes meaningful — callers want SELECT COUNT /
/// SUM / etc. for per-group output. Pin the discriminator so a
/// future refactor that collapses this back into the
/// not-yet-implemented family fails loudly.
#[test]
fn reject_group_by_with_documents_as_invalid_argument() {
    let request = GetDocumentsRequestV1 {
        selects: select_documents(),
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    match validate_and_route_for_tests(&request, &[], PlatformVersion::latest()) {
        Err(QueryError::InvalidArgument(msg)) => {
            assert!(
                msg.contains("GROUP BY with SELECT DOCUMENTS")
                    && msg.contains("not a valid SQL shape"),
                "expected SQL-shape-mismatch message, got: {msg}"
            );
        }
        Err(QueryError::Query(QuerySyntaxError::Unsupported(msg))) => panic!(
            "expected InvalidArgument for GROUP BY + SELECT DOCUMENTS; got \
             not_yet_implemented(\"{msg}\"). The two error classes carry different \
             contracts (malformed input vs. future capability) and must not be \
             collapsed — GROUP BY + DOCUMENTS is structurally invalid, not \
             future capability."
        ),
        other => panic!("expected InvalidArgument, got {:?}", other),
    }
}

#[test]
fn reject_group_by_field_not_in_where_clauses() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
        "GROUP BY on field 'color' which is not constrained",
    );
}

#[test]
fn reject_group_by_more_than_two_fields() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()),
        "GROUP BY with more than two fields",
    );
}

#[test]
fn reject_two_field_group_by_outside_compound_shape() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["color".to_string(), "brand".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        },
    ];
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()),
        "two-field GROUP BY outside the `(In, range)` compound shape",
    );
}

#[test]
fn accept_count_with_empty_group_by_routes_to_aggregate() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        ..empty_v1_request()
    };
    assert_eq!(
        validate_and_route_for_tests(&request, &[], PlatformVersion::latest()).unwrap(),
        "count_aggregate"
    );
}

#[test]
fn reject_count_aggregate_with_limit() {
    // Aggregate count is a single row; a `limit` is structurally
    // meaningless and previously caused Drive's per-In fan-out
    // to honor it and return a partial sum disguised as a total.
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        limit: Some(1),
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: platform_value!([30u32, 40u32]),
    }];
    match validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()) {
        Err(QueryError::Query(QuerySyntaxError::InvalidLimit(msg))) => {
            // The aggregate-mode limit rejection is now produced by the
            // shared `compute_aggregate_mode_and_check_limit` helper
            // (covers COUNT / SUM / AVG); the wording dropped the
            // function-specific "count" word from the parenthetical
            // since the helper is keyed off `function_name` (interpolated
            // as "SELECT COUNT" / "SELECT SUM" / "SELECT AVG" at the
            // head of the message). Assert against the function-agnostic
            // tail.
            assert!(
                msg.contains("SELECT COUNT with empty GROUP BY")
                    && msg.contains("aggregate is a single row"),
                "expected aggregate-count limit-rejection message, got: {msg}"
            );
        }
        other => panic!("expected InvalidLimit, got {other:?}"),
    }
}

#[test]
fn reject_count_group_by_in_with_limit() {
    // GROUP BY on an `In` field returns at most `|In|` entries
    // (capped at 100 by `WhereClause::in_values()`). A `limit`
    // is either redundant (≤ 100) or would silently truncate
    // the proof to fewer In branches than requested — the
    // PointLookupProof path can't represent a partial-In
    // selection in its `SizedQuery`, so the limit gets dropped
    // before reaching the path-query builder. Reject upstream
    // to make the contract explicit.
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["age".to_string()],
        limit: Some(1),
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: platform_value!([30u32, 40u32, 50u32]),
    }];
    match validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()) {
        Err(QueryError::Query(QuerySyntaxError::InvalidLimit(msg))) => {
            assert!(
                msg.contains("bounded by the In array"),
                "expected GroupByIn limit-rejection message, got: {msg}"
            );
        }
        other => panic!("expected InvalidLimit, got {other:?}"),
    }
}

#[test]
fn accept_single_field_group_by_on_in_field_with_range_routes_to_in_entries() {
    // `group_by=[in_field]` with an additional range clause is
    // valid: drive's `detect_mode` picks
    // `RangeAggregateCarrierProof` (grovedb #663) on the prove
    // path and `RangeNoProof` per-In-branch on the no-prove path —
    // both produce entries that line up with the caller's
    // single-field GROUP BY shape.
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["brand".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        },
    ];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()).unwrap(),
        "count_entries_via_in_field"
    );
}

#[test]
fn accept_single_field_group_by_on_range_field_with_in_routes_to_range_entries() {
    // Mirror of the above: `group_by=[range_field]` with an
    // active In on the prefix routes to
    // `CountMode::GroupByRange`, and drive picks
    // `RangeDistinctProof` (with In-fanout via subquery) on the
    // prove path or `RangeNoProof` distinct on the no-prove
    // path.
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        },
    ];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()).unwrap(),
        "count_entries_via_range_field"
    );
}

/// Routing decision must not depend on `where_clauses` element
/// order when two range clauses are present. Pins the
/// `is_range_field` / `is_in_field` membership-test contract
/// (match-any, not match-first) so a future refactor that swaps
/// back to a `.find(...).map(...) == Some(...)` shape fails
/// loudly rather than re-introducing the bug.
///
/// Drive's executor explicitly supports the two-range
/// `GroupByRange + prove` shape (see
/// `outer_range_plus_inner_range_with_prove_and_group_by_range_routes_to_carrier_proof`
/// in `rs-drive`); the router must reach it regardless of
/// which range clause the caller wrote first.
#[test]
fn group_by_routing_is_independent_of_two_range_clause_order() {
    let make_request = |where_clauses: Vec<WhereClause>| {
        let request = GetDocumentsRequestV1 {
            selects: select_count_star(),
            group_by: vec!["brand".to_string()],
            ..empty_v1_request()
        };
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()).unwrap()
    };

    let brand_range = WhereClause {
        field: "brand".to_string(),
        operator: WhereOperator::GreaterThan,
        value: platform_value!("acme"),
    };
    let color_range = WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: platform_value!("blue"),
    };

    // GROUP BY brand: both orderings must route the same way.
    assert_eq!(
        make_request(vec![brand_range.clone(), color_range.clone()]),
        "count_entries_via_range_field",
        "GROUP BY brand routing must not depend on whether brand or color is first",
    );
    assert_eq!(
        make_request(vec![color_range, brand_range]),
        "count_entries_via_range_field",
        "GROUP BY brand routing must not depend on whether brand or color is first",
    );
}

#[test]
fn accept_count_group_by_in_field_routes_to_in_entries() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["brand".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "brand".to_string(),
        operator: WhereOperator::In,
        value: platform_value!(["acme", "contoso"]),
    }];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()).unwrap(),
        "count_entries_via_in_field"
    );
}

#[test]
fn accept_count_group_by_range_field_routes_to_range_entries() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: platform_value!("blue"),
    }];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()).unwrap(),
        "count_entries_via_range_field"
    );
}

#[test]
fn accept_count_group_by_compound_routes_to_compound_entries() {
    let request = GetDocumentsRequestV1 {
        selects: select_count_star(),
        group_by: vec!["brand".to_string(), "color".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![
        WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        },
        WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        },
    ];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses, PlatformVersion::latest()).unwrap(),
        "count_entries_via_compound"
    );
}

#[test]
fn e2e_documents_select_matches_v0() {
    use dpp::data_contract::DataContractFactory;

    const PROTOCOL_VERSION_V12: u32 = 12;

    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let platform_version = PlatformVersion::latest();

    let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "color": {"type": "string", "position": 0, "maxLength": 32},
        },
        "indices": [{
            "name": "byColor",
            "properties": [{"color": "asc"}],
        }],
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "widget": document_schema });
    let contract = factory
        .create_with_value_config(
            dpp::tests::utils::generate_random_identifier_struct(),
            0,
            schemas,
            None,
            None,
        )
        .expect("create contract")
        .data_contract_owned();
    store_data_contract(&platform, &contract, version);

    let document_type = contract.document_type_for_name("widget").expect("widget");
    for i in 1..=3u8 {
        let doc = document_type
            .random_document(Some(i as u64), platform_version)
            .expect("random doc");
        store_document(&platform, &contract, document_type, &doc, platform_version);
    }

    // v0 baseline.
    let request_v0 = GetDocumentsRequestV0 {
        data_contract_id: contract.id().to_vec(),
        document_type: "widget".to_string(),
        r#where: Vec::new(),
        order_by: Vec::new(),
        limit: 0,
        prove: false,
        start: None,
    };
    let v0_result = platform
        .query_documents_v0(request_v0, &state, version)
        .expect("v0 query");
    let v0_docs = match v0_result.data {
        Some(r) => match r.result {
            Some(get_documents_response_v0::Result::Documents(d)) => d.documents,
            other => panic!("v0: expected Documents, got {:?}", other),
        },
        None => panic!("v0: empty data"),
    };
    assert_eq!(v0_docs.len(), 3);

    // v1 equivalent.
    let request_v1 = GetDocumentsRequestV1 {
        data_contract_id: contract.id().to_vec(),
        document_type: "widget".to_string(),
        where_clauses: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: None,
        prove: false,
        selects: select_documents(),
        group_by: Vec::new(),
        having: Vec::new(),
        offset: None,
    };
    let v1_result = platform
        .query_documents_v1(request_v1, &state, version)
        .expect("v1 query");
    let v1_docs = match v1_result.data {
        Some(r) => match r.result {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Documents(d)),
            })) => d.documents,
            other => panic!("v1: expected Documents, got {:?}", other),
        },
        None => panic!("v1: empty data"),
    };
    assert_eq!(v1_docs, v0_docs, "v0 and v1 returned the same documents");
}

#[test]
fn e2e_having_rejection_surfaces_in_response() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let request = GetDocumentsRequestV1 {
        data_contract_id: vec![0u8; 32],
        document_type: "anything".to_string(),
        where_clauses: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: None,
        prove: false,
        selects: select_count_star(),
        group_by: Vec::new(),
        having: vec![hc(
            having_aggregate::Function::Sum,
            "amount",
            having_clause::Operator::GreaterThan,
            Value::U64(100),
        )],
        offset: None,
    };
    let result = platform
        .query_documents_v1(request, &state, version)
        .expect("query call should not error at the transport layer");
    assert!(
        !result.errors.is_empty(),
        "expected validation error for HAVING request"
    );
    match &result.errors[0] {
        QueryError::Query(QuerySyntaxError::Unsupported(msg)) => {
            assert!(
                msg.contains("HAVING") && msg.contains("not yet implemented"),
                "expected HAVING-specific message, got: {}",
                msg
            );
        }
        other => panic!("expected Unsupported error, got {:?}", other),
    }
}

#[test]
fn reject_start_with_select_count() {
    let (platform, state, version) = setup_platform(None, Network::Testnet, None);
    let request = GetDocumentsRequestV1 {
        data_contract_id: vec![0u8; 32],
        document_type: "widget".to_string(),
        where_clauses: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: Some(V1Start::StartAfter(vec![1u8; 32])),
        prove: false,
        selects: select_count_star(),
        group_by: Vec::new(),
        having: Vec::new(),
        offset: None,
    };
    let result = platform
        .query_documents_v1(request, &state, version)
        .expect("query call should not error at the transport layer");
    assert!(!result.errors.is_empty(), "expected validation error");
    match &result.errors[0] {
        QueryError::Query(QuerySyntaxError::Unsupported(msg)) => {
            assert!(
                msg.contains("start_after") && msg.contains("not yet implemented"),
                "expected start_after-specific message, got: {}",
                msg
            );
        }
        other => panic!("expected Unsupported error, got {:?}", other),
    }
}

mod ported_v0_count_tests {
    //! Integration tests ported from the (now-removed)
    //! `document_count_query::v0` test module — exercises every count
    //! shape that the v0 endpoint exposed, now through the v1
    //! handler. Mechanical 1:1 translation: the request type changes
    //! from `GetDocumentsCountRequestV0` to `GetDocumentsRequestV1`
    //! with `select=COUNT` and the `return_distinct_counts_in_range`
    //! flag mapped to an explicit `group_by`; the response pattern
    //! changes from `GetDocumentsCountResponseV0`'s
    //! `Counts(CountResults { … })` envelope to v1's nested
    //! `Data(ResultData { variant: Counts(CountResults { … }) })`.
    //!
    //! Same fixtures + assertions as before — these tests are the
    //! load-bearing coverage for the entire count-execution surface
    //! and the port preserves them verbatim under the new wire shape.

    // `super` is the outer `tests` module (this file's top level);
    // `super::super` is `v1/mod.rs`. Reach v1 items through the latter
    // so the inner module sees `validate_and_route_for_tests`,
    // `GetDocumentsRequestV1`, etc. directly.
    use super::super::*;
    use super::{oc, select_count_star, select_documents, wc};
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select as V1Select;
    use dapi_grpc::platform::v0::get_documents_request::{
        OrderClause as ProtoOrderClause, WhereClause as ProtoWhereClause,
        WhereOperator as ProtoWhereOperator,
    };
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::DocumentV0Setters;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// Builds an in-memory v12 contract with a `widget` document
    /// type that has `documentsCountable: true` — the type's
    /// primary-key tree becomes a CountTree, enabling the
    /// unfiltered total-count fast path on both no-proof and prove
    /// paths.
    fn build_documents_countable_widget_contract() -> dpp::prelude::DataContract {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;
        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "documentsCountable": true,
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned()
    }

    pub(super) fn store_person_document(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        data_contract: &dpp::prelude::DataContract,
        id: [u8; 32],
        first_name: &str,
        last_name: &str,
        age: u64,
        platform_version: &PlatformVersion,
    ) {
        use dpp::document::{Document, DocumentV0};
        use std::collections::BTreeMap;

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut properties = BTreeMap::new();
        properties.insert("firstName".to_string(), Value::Text(first_name.to_string()));
        properties.insert("lastName".to_string(), Value::Text(last_name.to_string()));
        properties.insert("age".to_string(), Value::U64(age));

        let document: Document = DocumentV0 {
            contract_version: None,
            id: Identifier::from(id),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        store_document(
            platform,
            data_contract,
            document_type,
            &document,
            platform_version,
        );
    }

    /// Build a `SELECT COUNT` v1 request with the given knobs. Keeps
    /// each test's body focused on the per-test setup + assertion.
    #[allow(clippy::too_many_arguments)]
    fn count_v1_request(
        data_contract_id: Vec<u8>,
        document_type: &str,
        where_clauses: Vec<ProtoWhereClause>,
        order_by: Vec<ProtoOrderClause>,
        group_by: Vec<String>,
        limit: Option<u32>,
        prove: bool,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id,
            document_type: document_type.to_string(),
            where_clauses,
            order_by,
            limit,
            start: None,
            prove,
            selects: select_count_star(),
            group_by,
            having: Vec::new(),
            offset: None,
        }
    }

    /// Match the inner `Data(ResultData { variant: Counts(CountResults
    /// { variant: AggregateCount(_) }) })` shape and return the count.
    /// Panics on any other response shape.
    fn unwrap_aggregate(response: GetDocumentsResponseV1) -> u64 {
        match response.result {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant:
                    Some(result_data::Variant::Counts(CountResults {
                        variant: Some(count_results::Variant::AggregateCount(total)),
                    })),
            })) => total,
            other => panic!("expected aggregate count result, got {:?}", other),
        }
    }

    /// Match the inner `Data(ResultData { variant: Counts(CountResults
    /// { variant: Entries(_) }) })` shape and return the entries.
    fn unwrap_entries(response: GetDocumentsResponseV1) -> Vec<CountEntry> {
        match response.result {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant:
                    Some(result_data::Variant::Counts(CountResults {
                        variant: Some(count_results::Variant::Entries(entries)),
                    })),
            })) => entries.entries,
            other => panic!("expected per-key entries result, got {:?}", other),
        }
    }

    /// Unfiltered total count via the `documentsCountable: true`
    /// fast path. Ported from v0-count's `test_documents_count_no_prove`.
    #[test]
    fn ported_documents_count_no_prove() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let contract = build_documents_countable_widget_contract();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for i in 1..=5u8 {
            let random_document = document_type
                .random_document(Some(i as u64), platform_version)
                .expect("expected to get random document");
            store_document(
                &platform,
                &contract,
                document_type,
                &random_document,
                platform_version,
            );
        }

        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            vec![],
            Vec::new(),
            /* group_by = */ Vec::new(),
            /* limit = */ None,
            /* prove = */ false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(
            unwrap_aggregate(result.data.expect("data")),
            5,
            "expected count of 5 documents"
        );
    }

    /// Empty contract → aggregate 0. Ported from
    /// `test_documents_count_empty_result`.
    #[test]
    fn ported_documents_count_empty_result() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let contract = build_documents_countable_widget_contract();
        store_data_contract(&platform, &contract, version);

        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            vec![],
            Vec::new(),
            Vec::new(),
            None,
            false,
        );
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(
            unwrap_aggregate(result.data.expect("data")),
            0,
            "expected count of 0 documents"
        );
    }

    /// `In` clause + per-In entries. The v0-count endpoint did this
    /// implicitly (any In → PerInValue → entries); v1 makes the
    /// grouping explicit via `group_by=["age"]`. Ported from
    /// `test_documents_count_with_in_operator`.
    #[test]
    fn ported_documents_count_with_in_operator() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        for (id, name, age) in [
            ([1u8; 32], "Alice", 30u64),
            ([2u8; 32], "Bob", 30),
            ([3u8; 32], "Carol", 30),
            ([4u8; 32], "Dave", 40),
            ([5u8; 32], "Eve", 40),
            ([6u8; 32], "Frank", 50),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                name,
                "Smith",
                age,
                platform_version,
            );
        }

        let where_clauses = vec![wc(
            "age",
            ProtoWhereOperator::In,
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        )];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            where_clauses,
            Vec::new(),
            vec!["age".to_string()],
            None,
            false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let entries = unwrap_entries(result.data.expect("data"));
        let total: u64 = entries.iter().map(|e| e.count).sum();
        assert_eq!(total, 5, "expected count of 5 (3 age=30 + 2 age=40)");
    }

    /// `In` clause + **empty** `group_by` (= aggregate). Drive's
    /// `detect_mode` sees `In + no range + no prove` and routes to
    /// `DocumentCountMode::PerInValue`, which emits
    /// `DocumentCountResponse::Entries(Vec<SplitCountEntry>)`. The
    /// v1 handler then folds those entries back into a single
    /// `AggregateCount(total)` at the `mode.is_aggregate()` branch
    /// of `dispatch_count_v1` (the `saturating_add` fold over
    /// per-In counts). This is the only response-shape
    /// transformation the v1 handler introduces, so it deserves a
    /// dedicated regression to lock the wire contract:
    ///
    /// - Wire-visible shape MUST be `AggregateCount(_)`, not the
    ///   `Entries(_)` variant the drive executor emitted upstream.
    ///   A regression that forgets the `mode.is_aggregate()` branch
    ///   (or routes `select=COUNT, group_by=[]` differently in
    ///   `validate_and_route`) would silently leak per-In rows on
    ///   the wire — invisible to the documents-shape tests above.
    /// - The folded total MUST equal the sum of per-In counts. A
    ///   regression that off-by-ones the fold, picks the wrong
    ///   accumulator, or silently picks a single branch's count
    ///   instead would still produce an `AggregateCount` of the
    ///   wrong magnitude.
    ///
    /// Pairs structurally with
    /// [`ported_documents_count_with_in_operator`] above: same
    /// fixture, same `where` clause, only `group_by` differs
    /// (`["age"]` → wire `Entries`; `[]` → wire
    /// `AggregateCount`). Together they pin both halves of the
    /// `(group_by × PerInValue-execution)` matrix.
    #[test]
    fn documents_count_with_in_operator_and_empty_group_by_collapses_to_aggregate() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        // Same fixture rows as `ported_documents_count_with_in_operator`
        // (3 × age=30, 2 × age=40, 1 × age=50). The In clause matches
        // 3+2 = 5 rows; only age=50 sits outside the In window.
        for (id, name, age) in [
            ([1u8; 32], "Alice", 30u64),
            ([2u8; 32], "Bob", 30),
            ([3u8; 32], "Carol", 30),
            ([4u8; 32], "Dave", 40),
            ([5u8; 32], "Eve", 40),
            ([6u8; 32], "Frank", 50),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                name,
                "Smith",
                age,
                platform_version,
            );
        }

        let where_clauses = vec![wc(
            "age",
            ProtoWhereOperator::In,
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        )];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            where_clauses,
            Vec::new(),
            /* group_by = */ Vec::new(),
            /* limit = */ None,
            /* prove = */ false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);

        // Load-bearing assertion #1: wire shape is the aggregate
        // variant, NOT the entries variant. `unwrap_aggregate`
        // panics if `result.variant` is `Entries(_)` — which is
        // exactly the regression we're guarding against.
        let total = unwrap_aggregate(result.data.expect("data"));

        // Load-bearing assertion #2: the folded total equals the
        // sum of per-In counts (3 × age=30 + 2 × age=40 = 5). A
        // wrong-accumulator regression that picks a single branch
        // (e.g. returns 3 or 2 instead of 5) still produces an
        // `AggregateCount` and would pass assertion #1 alone.
        assert_eq!(
            total, 5,
            "expected aggregate count 5 (3 × age=30 + 2 × age=40); a value of 3 or 2 \
             indicates the per-In fold picks a single branch instead of summing"
        );
    }

    /// Range without a `range_countable` index → picker rejection.
    /// Ported from
    /// `test_documents_count_range_without_range_countable_index_returns_clear_error`.
    #[test]
    fn ported_range_without_range_countable_index_returns_clear_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        let where_clauses = vec![wc("age", ProtoWhereOperator::GreaterThan, Value::U64(20))];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            where_clauses,
            Vec::new(),
            Vec::new(),
            None,
            false,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to return validation error");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::InvalidArgument(msg)] if msg.contains("range_countable")
            ) || matches!(
                result.errors.as_slice(),
                [QueryError::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(msg))]
                    if msg.contains("range_countable")
            ),
            "expected range_countable-index rejection, got {:?}",
            result.errors
        );
    }

    /// `prove = true` + Equal-on-single-property-countable-index →
    /// CountTree element proof. Ported from
    /// `test_documents_count_with_prove_and_covering_equal`.
    #[test]
    fn ported_documents_count_with_prove_and_covering_equal() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(500);
        for first_name in ["Alice", "Alice", "Bob"] {
            let mut doc = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");
            let mut props = std::collections::BTreeMap::new();
            props.insert("firstName".to_string(), Value::Text(first_name.to_string()));
            props.insert("lastName".to_string(), Value::Text("Smith".to_string()));
            props.insert("age".to_string(), Value::U64(30));
            doc.set_properties(props);
            store_document(
                &platform,
                &data_contract,
                document_type,
                &doc,
                platform_version,
            );
        }

        let where_clauses = vec![wc(
            "firstName",
            ProtoWhereOperator::Equal,
            Value::Text("Alice".to_string()),
        )];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            where_clauses,
            Vec::new(),
            Vec::new(),
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for covered prove count"
                );
            }
            other => panic!("expected Proof response, got {:?}", other),
        }
    }

    /// `prove = true` with no covering index → clear error. Ported
    /// from `test_documents_count_prove_without_covering_index_returns_clear_error`.
    #[test]
    fn ported_prove_without_covering_index_returns_clear_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            vec![],
            Vec::new(),
            Vec::new(),
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to surface a validation error");
        assert!(
            matches!(
                result.errors.as_slice(),
                [QueryError::Query(
                    QuerySyntaxError::WhereClauseOnNonIndexedProperty(msg),
                )] if msg.contains("countable")
            ),
            "expected covering-index rejection, got {:?}",
            result.errors
        );
    }

    /// `prove = true` + `In` → CountTree element proof. Ported
    /// from `test_documents_count_with_in_and_prove_returns_proof`.
    /// v1 expresses the per-In emission explicitly via
    /// `group_by=["age"]`; the underlying drive routing decision
    /// (PointLookupProof) and emitted proof bytes are the same as
    /// the v0-count test.
    #[test]
    fn ported_documents_count_with_in_and_prove_returns_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        for (id, name, age) in [
            ([1u8; 32], "Alice", 30u64),
            ([2u8; 32], "Bob", 30),
            ([3u8; 32], "Carol", 30),
            ([4u8; 32], "Dave", 40),
            ([5u8; 32], "Eve", 40),
            ([6u8; 32], "Frank", 50),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                name,
                "Smith",
                age,
                platform_version,
            );
        }

        let where_clauses = vec![wc(
            "age",
            ProtoWhereOperator::In,
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        )];
        let order_by = vec![oc("age", /* ascending = */ true)];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            where_clauses,
            order_by,
            vec!["age".to_string()],
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("expected query to succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for In + prove count"
                );
            }
            other => panic!(
                "expected Proof response from In + prove count, got {:?}",
                other
            ),
        }
    }

    /// Range count happy path — sum + distinct + limit + direction.
    /// Ported from `test_documents_count_range_query_no_prove`. v1
    /// translates `return_distinct_counts_in_range=true` to
    /// `group_by=["color"]` and the summed mode keeps `group_by=[]`.
    #[test]
    fn ported_documents_count_range_query_no_prove() {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");

        for (i, color) in ["red", "red", "blue", "green", "green", "green"]
            .iter()
            .enumerate()
        {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), platform_version)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        let make_request = |group_by: Vec<String>, limit: Option<u32>, ascending: Option<bool>| {
            let where_clauses = vec![wc(
                "color",
                ProtoWhereOperator::GreaterThan,
                Value::Text("blue".to_string()),
            )];
            let order_by = match ascending {
                Some(asc) => vec![oc("color", asc)],
                None => Vec::new(),
            };
            count_v1_request(
                contract.id().to_vec(),
                "widget",
                where_clauses,
                order_by,
                group_by,
                limit,
                false,
            )
        };

        // Sum mode: green(3) + red(2) = 5.
        let result = platform
            .query_documents_v1(make_request(Vec::new(), None, None), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(unwrap_aggregate(result.data.expect("data")), 5);

        // Distinct mode ascending: [(green, 3), (red, 2)].
        let result = platform
            .query_documents_v1(
                make_request(vec!["color".to_string()], None, Some(true)),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let entries = unwrap_entries(result.data.expect("data"));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, b"green".to_vec());
        assert_eq!(entries[0].count, 3);
        assert_eq!(entries[1].key, b"red".to_vec());
        assert_eq!(entries[1].count, 2);

        // Distinct with limit=1.
        let result = platform
            .query_documents_v1(
                make_request(vec!["color".to_string()], Some(1), Some(true)),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        let entries = unwrap_entries(result.data.expect("data"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, b"green".to_vec());

        // Distinct descending: [(red, 2), (green, 3)].
        let result = platform
            .query_documents_v1(
                make_request(vec!["color".to_string()], None, Some(false)),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty());
        let entries = unwrap_entries(result.data.expect("data"));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, b"red".to_vec());
        assert_eq!(entries[1].key, b"green".to_vec());
    }

    /// `RangeDistinctProof` dispatch — `group_by=["color"]` +
    /// `prove=true` + range clause. Ported from
    /// `test_documents_count_range_with_prove_and_distinct_returns_proof`.
    #[test]
    fn ported_documents_count_range_with_prove_and_distinct_returns_proof() {
        use dpp::data_contract::DataContractFactory;
        use dpp::platform_value::platform_value;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let factory =
            DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();
        store_data_contract(&platform, &contract, version);

        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget exists");
        let platform_version = PlatformVersion::latest();
        for (i, color) in ["red", "red", "green", "green", "green", "blue"]
            .iter()
            .enumerate()
        {
            let mut doc = document_type
                .random_document(Some((i + 1) as u64), platform_version)
                .expect("random doc");
            let mut props = std::collections::BTreeMap::new();
            props.insert("color".to_string(), Value::Text(color.to_string()));
            doc.set_properties(props);
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        let where_clauses = vec![wc(
            "color",
            ProtoWhereOperator::GreaterThan,
            Value::Text("blue".to_string()),
        )];
        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            where_clauses,
            Vec::new(),
            vec!["color".to_string()],
            None,
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query should succeed");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for non-empty range result"
                );
            }
            other => panic!("expected Proof response, got {:?}", other),
        }
    }
}

mod ranked_tests {
    //! End-to-end coverage of the ranked
    //! (`GROUP BY p ORDER BY <the selected aggregate> LIMIT n OFFSET m`)
    //! surface through the real v1 handler: wire request in,
    //! `ResultData.ranked` (or a `Proof`) out.
    //!
    //! ## Fixture
    //!
    //! Shares rs-drive's `restaurants` fixture by path rather than
    //! copying it into this crate's `tests/supporting_files`. Both
    //! choices have precedent here (`crypto-card-game` is local,
    //! `family-contract` is referenced across crates from
    //! `perform_events_on_first_block_of_protocol_change`), and the
    //! cross-crate reference is the right one for this fixture: the
    //! drive-level ranked suite asserts the *values* this contract's
    //! indexes produce, and this suite asserts the *wire encoding* of
    //! those same values. A second copy could drift, and the drift
    //! would show up as one suite passing on a contract the other
    //! doesn't test.
    //!
    //! Its three doctypes give one per ranking axis — two ranked
    //! indexes over the same property on one doctype would be a
    //! `DuplicateIndexError`:
    //!
    //! | doctype  | index                | axis  | aggregated property |
    //! |----------|----------------------|-------|---------------------|
    //! | `review` | `byRestaurant`       | Avg   | `grade`             |
    //! | `visit`  | `byRestaurantVisits` | Count | — (`COUNT(*)`)      |
    //! | `tip`    | `byRestaurantTips`   | Sum   | `amount`            |

    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::json_document_to_contract;
    use drive::query::{RANKED_AVG_SCALE, RANKED_COUNT_ORDER_KEY};
    use std::collections::BTreeMap;

    /// Shared with rs-drive's ranked suite — see the module docs for
    /// why this is a cross-crate path and not a copy.
    pub(super) const RESTAURANTS_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/restaurants/restaurants-contract.json";

    /// The one property every fixture doctype groups by.
    pub(super) const GROUP_PROPERTY: &str = "restaurantId";

    /// The last protocol version whose query table (v0) has no ranked
    /// path at all. Ranked routing activates at 14.
    pub(super) const PROTOCOL_VERSION_V13: u32 = 13;

    pub(super) fn register_restaurants(
        platform: &Platform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let contract =
            json_document_to_contract(RESTAURANTS_CONTRACT_PATH, false, platform_version)
                .expect("expected to parse the restaurants contract");
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    /// Insert `(restaurant, value)` rows as documents through the
    /// ordinary document-insert path, so the ranked secondaries are
    /// maintained by the real write path rather than being fabricated.
    ///
    /// `first_seed` seeds the random-document generator, which derives
    /// each document's id; two calls in one test need disjoint seed
    /// ranges or the second collides on an existing id.
    pub(super) fn insert_docs(
        platform: &Platform<MockCoreRPCLike>,
        contract: &DataContract,
        document_type_name: &str,
        aggregated_property: &str,
        first_seed: u64,
        rows: &[(&str, i64)],
        platform_version: &PlatformVersion,
    ) {
        let document_type = contract
            .document_type_for_name(document_type_name)
            .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
        for (i, (restaurant, value)) in rows.iter().enumerate() {
            let mut document: Document = document_type
                .random_document(Some(first_seed + i as u64), platform_version)
                .expect("random document");
            let mut properties = BTreeMap::new();
            properties.insert(
                GROUP_PROPERTY.to_string(),
                Value::Text(restaurant.to_string()),
            );
            properties.insert(aggregated_property.to_string(), Value::I64(*value));
            document.set_properties(properties);
            store_document(
                platform,
                contract,
                document_type,
                &document,
                platform_version,
            );
        }
    }

    pub(super) fn select(function: v1_select::Function, field: &str) -> Vec<V1Select> {
        vec![V1Select {
            function: function as i32,
            field: field.to_string(),
        }]
    }

    /// The canonical ranked request: one aggregate select, one
    /// `group_by` on the ranked index's property, one `order_by`
    /// naming the selected aggregate, and a `limit`. Everything else is
    /// left at its "unset" wire value — which is exactly what a ranked
    /// request must do.
    ///
    /// `order_field` is the select's own field for `SUM` / `AVG`, and
    /// the `$count` sentinel for `COUNT(*)`.
    #[allow(clippy::too_many_arguments)]
    fn ranked_request(
        contract: &DataContract,
        document_type: &str,
        selects: Vec<V1Select>,
        order_field: &str,
        ascending: bool,
        limit: Option<u32>,
        offset: Option<u32>,
        prove: bool,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type: document_type.to_string(),
            where_clauses: Vec::new(),
            order_by: vec![oc(order_field, ascending)],
            limit,
            start: None,
            prove,
            selects,
            group_by: vec![GROUP_PROPERTY.to_string()],
            having: Vec::new(),
            offset,
        }
    }

    /// `ranked_request` for the common "highest first, no offset,
    /// no proof" shape.
    fn ranked_desc(
        contract: &DataContract,
        document_type: &str,
        selects: Vec<V1Select>,
        order_field: &str,
        limit: u32,
    ) -> GetDocumentsRequestV1 {
        ranked_request(
            contract,
            document_type,
            selects,
            order_field,
            false,
            Some(limit),
            None,
            false,
        )
    }

    /// Run a request through the handler and unwrap the whole ranked
    /// **page** — entries plus the `skipped` rank base — asserting the
    /// response landed on the `ranked` variant of `ResultData` rather
    /// than on `counts` / `sums` / `averages`.
    pub(super) fn ranked_page(
        platform: &Platform<MockCoreRPCLike>,
        state: &PlatformState,
        request: GetDocumentsRequestV1,
        platform_version: &PlatformVersion,
    ) -> RankedEntries {
        let result = platform
            .query_documents_v1(request, state, platform_version)
            .expect("query call should not error at the transport layer");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result:
                    Some(get_documents_response_v1::Result::Data(ResultData {
                        variant: Some(result_data::Variant::Ranked(ranked)),
                    })),
                metadata: Some(_),
            }) => ranked,
            other => panic!("expected a Ranked ResultData, got {:?}", other),
        }
    }

    /// [`ranked_page`] for the majority of tests, which only care about
    /// the entries.
    pub(super) fn ranked_entries(
        platform: &Platform<MockCoreRPCLike>,
        state: &PlatformState,
        request: GetDocumentsRequestV1,
        platform_version: &PlatformVersion,
    ) -> Vec<RankedEntry> {
        ranked_page(platform, state, request, platform_version).entries
    }

    /// The first validation error a request produces. Ranked
    /// rejections must arrive this way — as a `QueryError` on the
    /// validation result, which the gRPC layer turns into
    /// `invalid_argument` — and never as the `Err` arm, which becomes
    /// an opaque internal error.
    pub(super) fn ranked_error(
        platform: &Platform<MockCoreRPCLike>,
        state: &PlatformState,
        request: GetDocumentsRequestV1,
        platform_version: &PlatformVersion,
    ) -> QueryError {
        let result = platform
            .query_documents_v1(request, state, platform_version)
            .expect(
                "a rejected ranked request must surface as a validation error, not as a \
                 transport-level / internal error",
            );
        assert!(
            !result.errors.is_empty(),
            "expected a validation error, got data: {:?}",
            result.data
        );
        result.errors.into_iter().next().expect("checked non-empty")
    }

    pub(super) fn group_keys(entries: &[RankedEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|entry| String::from_utf8(entry.key.clone()).expect("fixture keys are utf-8"))
            .collect()
    }

    fn counts(entries: &[RankedEntry]) -> Vec<u64> {
        entries
            .iter()
            .map(|entry| match entry.value {
                Some(ranked_entry::Value::Count(count)) => count,
                ref other => panic!("expected a count value, got {:?}", other),
            })
            .collect()
    }

    fn sums(entries: &[RankedEntry]) -> Vec<i64> {
        entries
            .iter()
            .map(|entry| match entry.value {
                Some(ranked_entry::Value::Sum(sum)) => sum,
                ref other => panic!("expected a sum value, got {:?}", other),
            })
            .collect()
    }

    /// Pull the wire's `avg` doubles out of the entries.
    ///
    /// The wire carries a `double` approximation rather than the exact
    /// fixed-point `i128` because these entries only exist on the
    /// no-proof path — the proof path reconstructs the exact integer
    /// from the grovedb proof. Expectations are therefore built by
    /// running the *same* conversion the server runs (see
    /// [`expected_avg`]), so the comparison is exact rather than
    /// epsilon-based.
    fn avgs(entries: &[RankedEntry]) -> Vec<f64> {
        entries
            .iter()
            .map(|entry| match entry.value {
                Some(ranked_entry::Value::Avg(avg)) => avg,
                ref other => panic!("expected an avg value, got {:?}", other),
            })
            .collect()
    }

    /// grovedb's own fixed-point average, recomputed from the
    /// **re-exported** scale rather than from a literal. The scale
    /// changed from 10^15 to 10^19 late in grovedb's development; a
    /// hardcoded expectation here would have passed against the old
    /// storage and lied about the new one.
    fn expected_avg_fixed_point(sum: i64, count: u64) -> i128 {
        (sum as i128)
            .saturating_mul(RANKED_AVG_SCALE)
            .div_euclid(count as i128)
    }

    /// The double the wire is expected to carry: the fixed point above,
    /// divided by the scale in `f64` — the exact computation
    /// `RankedEntryValue::as_f64` performs server-side. Reproducing the
    /// rounding model rather than hardcoding a decimal literal is what
    /// lets these tests compare with `==`.
    fn expected_avg(sum: i64, count: u64) -> f64 {
        (expected_avg_fixed_point(sum, count) as f64) / (RANKED_AVG_SCALE as f64)
    }

    /// `SELECT COUNT(*) GROUP BY restaurantId ORDER BY $count DESC
    /// LIMIT 2` — the Count axis ranks groups by how many documents
    /// they hold, and `$count` is how a projection with no field of its
    /// own is named on the ordering surface.
    #[test]
    fn count_axis_ordered_by_the_count_sentinel_returns_ranked_entries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "visit",
            "guests",
            1,
            &[
                ("alpha", 2),
                ("beta", 4),
                ("beta", 2),
                ("beta", 6),
                ("gamma", 3),
                ("gamma", 1),
                ("delta", 5),
                ("delta", 5),
                ("delta", 5),
                ("delta", 5),
            ],
            version,
        );

        let entries = ranked_entries(
            &platform,
            &state,
            ranked_desc(
                &contract,
                "visit",
                select(v1_select::Function::Count, ""),
                RANKED_COUNT_ORDER_KEY,
                2,
            ),
            version,
        );

        assert_eq!(
            group_keys(&entries),
            vec!["delta", "beta"],
            "descending by document count: delta(4) > beta(3) > gamma(2) > alpha(1)"
        );
        assert_eq!(counts(&entries), vec![4, 3]);
    }

    /// A `COUNT(*)` request that orders by a *schema* property instead
    /// of the `$count` sentinel is not a ranked request — it is an
    /// ordinary grouped count, and must route as one. This is the
    /// discriminator the whole routing decision rests on, so it is
    /// pinned from the wire rather than from the drive-side unit test.
    #[test]
    fn count_ordered_by_a_schema_property_is_not_ranked() {
        let base = GetDocumentsRequestV1 {
            group_by: vec![GROUP_PROPERTY.to_string()],
            selects: select(v1_select::Function::Count, ""),
            ..empty_v1_request()
        };
        let where_clauses = [WhereClause {
            field: GROUP_PROPERTY.to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("a"),
        }];

        // `ORDER BY restaurantId` — a real column, not the aggregate.
        let by_property = GetDocumentsRequestV1 {
            order_by: vec![oc(GROUP_PROPERTY, true)],
            ..base.clone()
        };
        assert_eq!(
            validate_and_route_for_tests(&by_property, &where_clauses, PlatformVersion::latest())
                .expect("a range-bound GROUP BY is a supported non-ranked shape"),
            "count_entries_via_range_field",
            "ordering by a document property leaves the request on the grouped path"
        );

        // A bare `count` column name is not the sentinel either.
        let by_lookalike = GetDocumentsRequestV1 {
            order_by: vec![oc("count", true)],
            ..base.clone()
        };
        assert_eq!(
            validate_and_route_for_tests(&by_lookalike, &where_clauses, PlatformVersion::latest())
                .expect("still the grouped path"),
            "count_entries_via_range_field",
        );

        // The sentinel is what flips it.
        let ranked = GetDocumentsRequestV1 {
            order_by: vec![oc(RANKED_COUNT_ORDER_KEY, false)],
            limit: Some(5),
            ..base
        };
        assert_eq!(
            validate_and_route_for_tests(&ranked, &where_clauses, PlatformVersion::latest())
                .expect("ORDER BY $count is the ranked shape"),
            "ranked",
        );
    }

    /// `SELECT SUM(amount) … ORDER BY amount DESC LIMIT 3` — the Sum
    /// axis ranks by the running total of the index's summable
    /// property, named on the ordering surface by that same field.
    #[test]
    fn sum_axis_ordered_by_the_summed_field_returns_ranked_entries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "tip",
            "amount",
            1,
            &[
                ("alpha", 10),
                ("alpha", 15),
                ("beta", 100),
                ("gamma", 7),
                ("gamma", 8),
                ("gamma", 9),
            ],
            version,
        );

        let entries = ranked_entries(
            &platform,
            &state,
            ranked_desc(
                &contract,
                "tip",
                select(v1_select::Function::Sum, "amount"),
                "amount",
                3,
            ),
            version,
        );

        assert_eq!(
            group_keys(&entries),
            vec!["beta", "alpha", "gamma"],
            "descending by summed amount: beta(100) > alpha(25) > gamma(24)"
        );
        assert_eq!(sums(&entries), vec![100, 25, 24]);
    }

    /// The headline shape: **top 5 restaurants by average grade**.
    /// `SELECT AVG(grade) GROUP BY restaurantId ORDER BY grade DESC
    /// LIMIT 5`. This is the exact request the SDK will encode, so the
    /// wire values are asserted field by field.
    #[test]
    fn avg_axis_top_k_returns_fixed_point_entries() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "review",
            "grade",
            1,
            &[
                ("alpha", 90),
                ("alpha", 80),
                ("beta", 60),
                ("beta", 70),
                ("beta", 50),
                ("gamma", 95),
                ("delta", 40),
                ("delta", 20),
                // 21/2 = 10.5 — a non-integral average, so the
                // fixed-point floor is actually exercised.
                ("epsilon", 10),
                ("epsilon", 11),
            ],
            version,
        );

        let entries = ranked_entries(
            &platform,
            &state,
            ranked_desc(
                &contract,
                "review",
                select(v1_select::Function::Avg, "grade"),
                "grade",
                5,
            ),
            version,
        );

        assert_eq!(
            group_keys(&entries),
            vec!["gamma", "alpha", "beta", "delta", "epsilon"],
            "descending by average grade: gamma(95) > alpha(85) > beta(60) > delta(30) \
             > epsilon(10.5)"
        );
        assert_eq!(
            avgs(&entries),
            vec![
                expected_avg(95, 1),
                expected_avg(170, 2),
                expected_avg(180, 3),
                expected_avg(60, 2),
                expected_avg(21, 2),
            ],
            "each entry is floor(sum * RANKED_AVG_SCALE / count) rendered as a double"
        );
        // And the double really is the average a caller would render —
        // 21/2 survives the fixed-point round trip exactly.
        assert_eq!(avgs(&entries)[4], 10.5);
    }

    /// **`OFFSET` is accepted on the ranked path**, which is the one
    /// place in the v1 surface where it is. `LIMIT 1 OFFSET 4` is how
    /// "the 5th best average grade" is asked for, and a window running
    /// past the end comes back short rather than erroring.
    #[test]
    fn offset_pages_through_a_ranking() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "review",
            "grade",
            1,
            &[
                ("gamma", 95),
                ("alpha", 85),
                ("beta", 60),
                ("delta", 30),
                ("epsilon", 10),
            ],
            version,
        );

        let paged = |limit, offset| {
            ranked_request(
                &contract,
                "review",
                select(v1_select::Function::Avg, "grade"),
                "grade",
                false,
                Some(limit),
                Some(offset),
                false,
            )
        };

        // The 5th best grade.
        let fifth = ranked_page(&platform, &state, paged(1, 4), version);
        assert_eq!(
            group_keys(&fifth.entries),
            vec!["epsilon"],
            "gamma > alpha > beta > delta > epsilon — rank 4 (0-based) is epsilon"
        );
        assert_eq!(avgs(&fifth.entries), vec![expected_avg(10, 1)]);
        assert_eq!(
            fifth.skipped,
            Some(4),
            "the page echoes the rank it starts at, which is what makes the single \
             entry identifiable as the *5th* best rather than the best"
        );

        // Rank 0 still reports a base, so a caller never has to guess
        // whether an absent `skipped` means "rank 0" or "old node".
        let best = ranked_page(&platform, &state, paged(1, 0), version);
        assert_eq!(group_keys(&best.entries), vec!["gamma"]);
        assert_eq!(best.skipped, Some(0));

        // A window that spans the end returns the tail, short.
        let tail = ranked_page(&platform, &state, paged(3, 3), version);
        assert_eq!(
            group_keys(&tail.entries),
            vec!["delta", "epsilon"],
            "only two groups remain from rank 3"
        );
        assert_eq!(tail.skipped, Some(3));

        // A window entirely past the end is an empty page, not an
        // error, and `skipped` collapses to the population the walk
        // actually reached. That reaches the wire on this *unproven*
        // path too: grovedb's counted descent tracks how far the skip
        // got and returns it on the page, so the response carries a
        // population rather than the offset that was requested.
        let past_end = ranked_page(&platform, &state, paged(2, 9), version);
        assert!(
            past_end.entries.is_empty(),
            "there is no rank 9 in a five-group ranking, and asking for one is not an \
             error — got {:?}",
            group_keys(&past_end.entries)
        );
        assert_eq!(
            past_end.skipped,
            Some(5),
            "the response reports the five groups the ranking holds, not the offset that \
             was asked for"
        );

        // And the same page proves.
        let result = platform
            .query_documents_v1(
                ranked_request(
                    &contract,
                    "review",
                    select(v1_select::Function::Avg, "grade"),
                    "grade",
                    false,
                    Some(1),
                    Some(4),
                    true,
                ),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "got {:?}", result.errors);
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                ..
            }) => assert!(!proof.grovedb_proof.is_empty()),
            other => panic!("expected a Proof response, got {:?}", other),
        }
    }

    /// **OFFSET stays refused everywhere else.** The relaxation is
    /// scoped to the ranked path and to nothing else: documents
    /// fetches and the grouped count / sum / average modes keep the
    /// exact `not_yet_implemented` message they have always returned,
    /// character for character, because clients match on it.
    #[test]
    fn offset_is_still_rejected_off_the_ranked_path() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        const EXPECTED: &str = "OFFSET pagination (use cursor pagination via \
                                `start_after` / `start_at` instead) is not yet implemented";

        // SELECT DOCUMENTS with an offset.
        let documents = GetDocumentsRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type: "review".to_string(),
            offset: Some(3),
            ..empty_v1_request()
        };
        match ranked_error(&platform, &state, documents, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert_eq!(
                    message, EXPECTED,
                    "the documents-path message must not drift"
                );
            }
            other => panic!("expected the OFFSET rejection, got {other:?}"),
        }

        // SELECT AVG with a GROUP BY but ordered by a document
        // property — a grouped aggregate, not a ranking. The range
        // where clause is what makes this an otherwise *routable*
        // grouped request, so the OFFSET gate is the first thing it
        // trips rather than the second.
        let grouped = GetDocumentsRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type: "review".to_string(),
            selects: select(v1_select::Function::Avg, "grade"),
            group_by: vec![GROUP_PROPERTY.to_string()],
            where_clauses: vec![wc(
                GROUP_PROPERTY,
                ProtoWhereOperator::GreaterThan,
                Value::Text("a".to_string()),
            )],
            order_by: vec![oc(GROUP_PROPERTY, true)],
            offset: Some(3),
            ..empty_v1_request()
        };
        match ranked_error(&platform, &state, grouped, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert_eq!(message, EXPECTED, "the grouped-path message must not drift");
            }
            other => panic!("expected the OFFSET rejection, got {other:?}"),
        }
    }

    /// `prove = true` takes the proof arm of the response envelope,
    /// exactly like the count / sum / average dispatchers — the
    /// ranked entries live inside the grovedb proof, not alongside it.
    #[test]
    fn ranked_request_with_prove_returns_a_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "review",
            "grade",
            1,
            &[("alpha", 90), ("beta", 60), ("gamma", 95)],
            version,
        );

        let result = platform
            .query_documents_v1(
                ranked_request(
                    &contract,
                    "review",
                    select(v1_select::Function::Avg, "grade"),
                    "grade",
                    false,
                    Some(5),
                    None,
                    true,
                ),
                &state,
                version,
            )
            .expect("query should succeed");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(_),
            }) => {
                assert!(
                    !proof.grovedb_proof.is_empty(),
                    "expected non-empty grovedb proof bytes for a non-empty ranking"
                );
            }
            other => panic!("expected a Proof response, got {:?}", other),
        }
    }

    /// `ASC` walks the axis from the smallest aggregate up, and
    /// `LIMIT 1` is how the single worst- (or, for `DESC`, best-)
    /// ranked group is asked for. Both still return *entries*, because
    /// "which group" is as much of the answer as "what value".
    #[test]
    fn ascending_order_returns_the_worst_ranked_groups() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "tip",
            "amount",
            1,
            &[("alpha", 10), ("beta", 100), ("gamma", 55)],
            version,
        );

        let sum_request = |ascending, limit| {
            ranked_request(
                &contract,
                "tip",
                select(v1_select::Function::Sum, "amount"),
                "amount",
                ascending,
                Some(limit),
                None,
                false,
            )
        };

        let bottom_two = ranked_entries(&platform, &state, sum_request(true, 2), version);
        assert_eq!(
            group_keys(&bottom_two),
            vec!["alpha", "gamma"],
            "ASC walks upward from the smallest sum"
        );
        assert_eq!(sums(&bottom_two), vec![10, 55]);

        let top_one = ranked_entries(&platform, &state, sum_request(false, 1), version);
        assert_eq!(
            group_keys(&top_one),
            vec!["beta"],
            "DESC LIMIT 1 is the single largest sum"
        );
        assert_eq!(sums(&top_one), vec![100]);

        let bottom_one = ranked_entries(&platform, &state, sum_request(true, 1), version);
        assert_eq!(
            group_keys(&bottom_one),
            vec!["alpha"],
            "ASC LIMIT 1 is the single smallest sum"
        );
        assert_eq!(sums(&bottom_one), vec![10]);
    }

    /// A single boolean `HAVING` clause alongside an aggregate ordering
    /// no longer rides the ranked path at all: the v2 routing helper
    /// sends any grouped single-clause `having` to the having-range
    /// surface, where an `ORDER BY` naming the selected aggregate is
    /// legal and sets the walk direction. This request — a client left
    /// in place across the capability landing, exactly as the old
    /// `not_yet_implemented` contract promised — now answers. Full
    /// having-range behaviour is pinned in [`super::having_range_tests`];
    /// this test pins the routing handoff from the ranked shape.
    #[test]
    fn having_alongside_an_aggregate_ordering_now_routes_to_having_range() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "review",
            "grade",
            9_000,
            &[("alpha", 90), ("beta", 30), ("gamma", 60)],
            version,
        );

        let mut request = ranked_desc(
            &contract,
            "review",
            select(v1_select::Function::Avg, "grade"),
            "grade",
            3,
        );
        request.having = vec![hc(
            having_aggregate::Function::Avg,
            "grade",
            having_clause::Operator::GreaterThan,
            Value::U64(40),
        )];

        let page = ranked_page(&platform, &state, request, version);
        assert_eq!(
            page.skipped, None,
            "a having-range page has no rank base and must leave `skipped` unset"
        );
        assert_eq!(
            group_keys(&page.entries),
            vec!["alpha", "gamma"],
            "descending walk over averages above 40: alpha (90) then gamma (60)"
        );
    }

    /// Ordering by the selected aggregate **without** a `GROUP BY` is
    /// not a ranking — there is one implicit group and nothing to rank
    /// it against — so the request stays on the plain aggregate path
    /// and is refused there for the reason that path refuses it.
    #[test]
    fn ordering_by_the_aggregate_without_group_by_is_not_ranked() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let mut request = ranked_desc(
            &contract,
            "review",
            select(v1_select::Function::Avg, "grade"),
            "grade",
            3,
        );
        request.group_by = Vec::new();

        let routing_error = validate_and_route_for_tests(&request, &[], version)
            .expect_err("an aggregate with a limit and no GROUP BY is refused")
            .to_string();
        assert!(
            routing_error.contains("limit"),
            "the plain-aggregate path owns this rejection, not the ranked one; got: \
             {routing_error}"
        );

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::InvalidLimit(message)) => {
                assert!(
                    message.contains("empty GROUP BY"),
                    "expected the aggregate-path limit rejection, got: {message}"
                );
            }
            other => panic!("expected InvalidLimit, got {other:?}"),
        }
    }

    /// `limit` is **required** on the ranked path — it is the `k` the
    /// proof envelope echoes, so there is no server default a verifying
    /// client could reproduce. What this pins is that drive's
    /// `Error::Query` reaches the caller as a query error on the
    /// validation result, rather than being swallowed into an internal
    /// error by the dispatcher's `Err(e) => Err(e.into())` arm.
    #[test]
    fn a_ranked_request_without_a_limit_surfaces_as_a_query_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let request = ranked_request(
            &contract,
            "review",
            select(v1_select::Function::Avg, "grade"),
            "grade",
            false,
            None,
            None,
            false,
        );

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::InvalidLimit(message)) => {
                assert!(
                    message.contains("require an explicit `limit`"),
                    "expected drive's own limit rejection to reach the caller verbatim, \
                     got: {message}"
                );
            }
            other => panic!("expected InvalidLimit from drive, got {other:?}"),
        }
    }

    /// A limit over the ranked ceiling is drive's rejection too, and
    /// reaches the caller the same way.
    #[test]
    fn a_ranked_limit_over_the_ceiling_surfaces_as_a_query_error() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let request = ranked_desc(
            &contract,
            "review",
            select(v1_select::Function::Avg, "grade"),
            "grade",
            101,
        );

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::InvalidLimit(message)) => {
                assert!(
                    message.contains("ceiling of 100"),
                    "expected drive's ceiling rejection, got: {message}"
                );
            }
            other => panic!("expected InvalidLimit from drive, got {other:?}"),
        }
    }

    /// **Empty rankings prove.** grovedb's paginated prover emits a
    /// guaranteed-empty range for an empty axis secondary rather than
    /// refusing, so a freshly registered contract queried with
    /// `prove = true` gets a proof instead of the "cannot prove an
    /// empty tree" error the non-paginated prover used to raise. The
    /// unproven read of the same request returns an empty list, so the
    /// two paths agree.
    #[test]
    fn proving_an_empty_ranking_succeeds() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let request = |prove| {
            ranked_request(
                &contract,
                "review",
                select(v1_select::Function::Avg, "grade"),
                "grade",
                false,
                Some(5),
                None,
                prove,
            )
        };

        let entries = ranked_entries(&platform, &state, request(false), version);
        assert!(
            entries.is_empty(),
            "an index with no documents has no groups to rank"
        );

        let result = platform
            .query_documents_v1(request(true), &state, version)
            .expect("query should succeed");
        assert!(
            result.errors.is_empty(),
            "proving an empty ranking must not be a rejection any more, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                ..
            }) => assert!(!proof.grovedb_proof.is_empty()),
            other => panic!("expected a Proof response, got {:?}", other),
        }

        // One document, and the identical request keeps proving.
        insert_docs(
            &platform,
            &contract,
            "review",
            "grade",
            1,
            &[("alpha", 42)],
            version,
        );
        let result = platform
            .query_documents_v1(request(true), &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "got {:?}", result.errors);
    }

    /// **The activation gate.** Protocol version 13's query table
    /// selects the v0 routing helper, which has no ranked path: it
    /// reads the request as an ordinary grouped aggregate and refuses
    /// it because the `GROUP BY` property carries no `In` or range
    /// where clause. A v13 node and a v14 node must disagree here and
    /// nowhere else, which is what lets a mixed-version network run
    /// through the upgrade.
    ///
    /// The contract is never fetched (the gate fires first), so this
    /// deliberately runs against a bare platform: registering the
    /// restaurants fixture under v13 would fail anyway, since
    /// meta-schema v2 rejects the `ranked*` keywords.
    #[test]
    fn protocol_version_13_does_not_route_ranked() {
        let (platform, state, version) =
            setup_platform(None, Network::Testnet, Some(PROTOCOL_VERSION_V13));
        assert_eq!(
            version.protocol_version, PROTOCOL_VERSION_V13,
            "this test is only meaningful on the v13 table"
        );

        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "review".to_string(),
            where_clauses: Vec::new(),
            order_by: vec![oc("grade", false)],
            limit: Some(5),
            start: None,
            prove: false,
            selects: select(v1_select::Function::Avg, "grade"),
            group_by: vec![GROUP_PROPERTY.to_string()],
            having: Vec::new(),
            offset: None,
        };

        match ranked_error(&platform, &state, request.clone(), version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("GROUP BY on field 'restaurantId'")
                        && message.contains("not yet implemented"),
                    "v13 must read a ranked-shaped request as an unsupported grouped \
                     aggregate, got: {message}"
                );
            }
            other => panic!("expected the v13 grouped rejection, got {other:?}"),
        }

        // And an OFFSET on v13 is refused exactly as it always was —
        // the relaxation is gated behind the ranked route, which v13
        // never takes.
        let with_offset = GetDocumentsRequestV1 {
            offset: Some(2),
            ..request
        };
        match ranked_error(&platform, &state, with_offset, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("GROUP BY on field 'restaurantId'"),
                    "routing still runs first on v13; the offset gate is downstream of \
                     it, got: {message}"
                );
            }
            other => panic!("expected the v13 grouped rejection, got {other:?}"),
        }
    }

    /// The routing decision itself, without a platform: an `ORDER BY`
    /// naming the selected aggregate routes to "ranked" on each
    /// aggregate select, and the same request ordered by a document
    /// property routes exactly where it did before. Guards the half of
    /// the v1 helper that is supposed to be a no-op.
    #[test]
    fn routing_picks_ranked_only_when_order_by_names_the_selected_aggregate() {
        let base = GetDocumentsRequestV1 {
            group_by: vec![GROUP_PROPERTY.to_string()],
            ..empty_v1_request()
        };
        let where_clauses = [WhereClause {
            field: GROUP_PROPERTY.to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("a"),
        }];

        for (selects, order_field) in [
            (
                select(v1_select::Function::Count, ""),
                RANKED_COUNT_ORDER_KEY,
            ),
            (select(v1_select::Function::Sum, "amount"), "amount"),
            (select(v1_select::Function::Avg, "grade"), "grade"),
        ] {
            let ranked = GetDocumentsRequestV1 {
                selects: selects.clone(),
                order_by: vec![oc(order_field, false)],
                limit: Some(5),
                ..base.clone()
            };
            assert_eq!(
                validate_and_route_for_tests(&ranked, &[], PlatformVersion::latest()).unwrap(),
                "ranked",
                "an ORDER BY on the selected aggregate routes to the ranked executor"
            );

            // Same select, ordered by a document property: unchanged
            // pre-ranked routing, driven by the where-clause shape as
            // before.
            let grouped = GetDocumentsRequestV1 {
                selects,
                order_by: vec![oc(GROUP_PROPERTY, true)],
                ..base.clone()
            };
            let label =
                validate_and_route_for_tests(&grouped, &where_clauses, PlatformVersion::latest())
                    .expect("a range-bound GROUP BY is a supported non-ranked shape");
            assert_ne!(
                label, "ranked",
                "ordering by a document property means no ranked routing"
            );

            // And no ORDER BY at all is likewise not ranked.
            let unordered = GetDocumentsRequestV1 {
                order_by: Vec::new(),
                ..grouped
            };
            let label =
                validate_and_route_for_tests(&unordered, &where_clauses, PlatformVersion::latest())
                    .expect("a range-bound GROUP BY is a supported non-ranked shape");
            assert_ne!(label, "ranked", "no ORDER BY means no ranked routing");
        }
    }
}

/// Wire-level coverage for multiple `In` clauses on a compound index
/// (protocol version 14+): the v1 documents select accepts them at
/// v14 on both the no-proof and prove paths, while a protocol
/// version 13 execution of the very same request rejects them with
/// `MultipleInClauses`.
mod multi_in_wire_tests {
    use super::ported_v0_count_tests::store_person_document;
    use super::*;
    use dpp::platform_value::platform_value;
    use dpp::tests::json_document::json_document_to_contract_with_ids;

    fn documents_v1_request(data_contract_id: Vec<u8>, prove: bool) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id,
            document_type: "person".to_string(),
            where_clauses: vec![
                wc(
                    "firstName",
                    ProtoWhereOperator::In,
                    platform_value!(["Alice", "Carol", "Eve"]),
                ),
                wc(
                    "lastName",
                    ProtoWhereOperator::In,
                    platform_value!(["Kriskov", "Smith"]),
                ),
            ],
            order_by: vec![oc("firstName", true), oc("lastName", true)],
            limit: None,
            start: None,
            prove,
            selects: select_documents(),
            group_by: Vec::new(),
            having: Vec::new(),
            offset: None,
        }
    }

    fn setup_people() -> (
        crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        std::sync::Arc<crate::platform_types::platform_state::PlatformState>,
        &'static PlatformVersion,
        dpp::prelude::DataContract,
    ) {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            version,
        )
        .expect("expected to get json based contract");
        store_data_contract(&platform, &data_contract, version);

        for (id, first_name, last_name) in [
            ([1u8; 32], "Alice", "Kriskov"),
            ([2u8; 32], "Alice", "Smith"),
            ([3u8; 32], "Bob", "Kriskov"),
            ([4u8; 32], "Carol", "Smith"),
            ([5u8; 32], "Eve", "Sojka"),
        ] {
            store_person_document(
                &platform,
                &data_contract,
                id,
                first_name,
                last_name,
                30,
                version,
            );
        }

        (platform, state, version, data_contract)
    }

    #[test]
    fn e2e_multiple_in_clauses_documents_select_at_v14() {
        let (platform, state, version, data_contract) = setup_people();
        assert!(
            version.protocol_version >= 14,
            "test platform should run at protocol version 14 or later"
        );

        let request = documents_v1_request(data_contract.id().to_vec(), false);
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let documents = match result.data.expect("data").result {
            Some(get_documents_response_v1::Result::Data(ResultData {
                variant: Some(result_data::Variant::Documents(documents)),
            })) => documents.documents,
            other => panic!("expected documents, got {:?}", other),
        };
        // Alice/Kriskov, Alice/Smith, Carol/Smith — Bob and Eve/Sojka
        // fall outside the cross product
        assert_eq!(documents.len(), 3);

        let request = documents_v1_request(data_contract.id().to_vec(), true);
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        match result.data.expect("data").result {
            Some(get_documents_response_v1::Result::Proof(proof)) => {
                assert!(!proof.grovedb_proof.is_empty(), "proof should not be empty");
            }
            other => panic!("expected proof, got {:?}", other),
        }
    }

    #[test]
    fn e2e_multiple_in_clauses_rejected_at_protocol_version_13() {
        let (platform, state, _, data_contract) = setup_people();
        let version_13 = PlatformVersion::get(13).expect("protocol version 13 should exist");

        let request = documents_v1_request(data_contract.id().to_vec(), false);
        let result = platform
            .query_documents_v1(request, &state, version_13)
            .expect("query call should not error at the transport layer");
        assert!(
            !result.errors.is_empty(),
            "expected a validation error at protocol version 13"
        );
        // The v0 (protocol version 13) grammar rejects the shape at parse
        // time, so the error surfaces through the parse wrapping — the
        // same wire shape historical nodes produced.
        match &result.errors[0] {
            QueryError::Drive(drive::error::Error::Query(QuerySyntaxError::MultipleInClauses(
                _,
            ))) => {}
            other => panic!("expected MultipleInClauses, got {:?}", other),
        }
    }
}

mod having_range_tests {
    //! End-to-end coverage of the having-range
    //! (`GROUP BY p HAVING <the selected aggregate> <op> <value> LIMIT n`)
    //! surface through the real v1 handler: wire request in,
    //! `ResultData.ranked` with `skipped` unset (or a `Proof`) out.
    //!
    //! Shares the `restaurants` fixture with [`super::ranked_tests`] —
    //! same cross-crate path, same doctype → axis table — because the
    //! having-range surface reads the very same indexed trees; only the
    //! addressing (value bound instead of rank) differs. Value-level
    //! behaviour (bounds translation, proof round-trips, tamper
    //! rejection) is pinned in rs-drive's
    //! `drive_document_having_query::tests`; this suite pins the wire
    //! encoding, the routing, and the rejection contracts.

    use super::ranked_tests::{
        group_keys, insert_docs, ranked_error, ranked_page, register_restaurants, select,
        GROUP_PROPERTY, PROTOCOL_VERSION_V13,
    };
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::tests::json_document::json_document_to_contract;
    use std::collections::BTreeMap;

    /// The canonical having-range request: one aggregate select, one
    /// `group_by`, one `having` clause on the selected aggregate, a
    /// `limit`, and optionally an `order_by` naming the selected
    /// aggregate. Everything else at its "unset" wire value.
    #[allow(clippy::too_many_arguments)]
    fn having_request(
        contract: &dpp::prelude::DataContract,
        document_type: &str,
        selects: Vec<V1Select>,
        clause: ProtoHavingClause,
        order_by: Vec<ProtoOrderClause>,
        limit: Option<u32>,
        prove: bool,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type: document_type.to_string(),
            where_clauses: Vec::new(),
            order_by,
            limit,
            start: None,
            prove,
            selects,
            group_by: vec![GROUP_PROPERTY.to_string()],
            having: vec![clause],
            offset: None,
        }
    }

    /// `SELECT COUNT(*) GROUP BY restaurantId HAVING $count > 2
    /// LIMIT 10` — the headline spam-resistant-discovery shape. No
    /// `order_by`: ascending by count is the default, and `skipped`
    /// must be unset because a value-bounded page has no rank base.
    #[test]
    fn count_threshold_returns_matching_entries_with_no_rank_base() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "visit",
            "guests",
            10_000,
            &[
                ("alpha", 1),
                ("beta", 1),
                ("beta", 2),
                ("beta", 3),
                ("gamma", 1),
                ("gamma", 2),
                ("delta", 1),
                ("delta", 2),
                ("delta", 3),
                ("delta", 4),
            ],
            version,
        );

        let request = having_request(
            &contract,
            "visit",
            select(v1_select::Function::Count, ""),
            hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(2),
            ),
            Vec::new(),
            Some(10),
            false,
        );

        let page = ranked_page(&platform, &state, request, version);
        assert_eq!(
            page.skipped, None,
            "a having-range page must leave the rank-based `skipped` field unset"
        );
        assert_eq!(
            group_keys(&page.entries),
            vec!["beta", "delta"],
            "ascending count order: beta (3 visits) before delta (4)"
        );
    }

    /// `prove = true` answers with a `Proof` payload, exactly like the
    /// ranked path. The proof's verifiability is pinned in rs-drive's
    /// suite; here only the wire shape is asserted.
    #[test]
    fn a_having_request_with_prove_returns_a_proof() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);
        insert_docs(
            &platform,
            &contract,
            "visit",
            "guests",
            11_000,
            &[("alpha", 1), ("beta", 1), ("beta", 2), ("beta", 3)],
            version,
        );

        let request = having_request(
            &contract,
            "visit",
            select(v1_select::Function::Count, ""),
            hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(2),
            ),
            Vec::new(),
            Some(10),
            true,
        );

        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(_)),
                metadata: Some(_),
            }) => {}
            other => panic!("expected a Proof result, got {:?}", other),
        }
    }

    /// Two clauses (implicit AND) keep the `not_yet_implemented`
    /// contract, with a message that names the restriction rather than
    /// the blanket "HAVING clause".
    #[test]
    fn multiple_clauses_are_still_not_implemented() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let clause = hc(
            having_aggregate::Function::Count,
            "",
            having_clause::Operator::GreaterThan,
            Value::U64(2),
        );
        let mut request = having_request(
            &contract,
            "visit",
            select(v1_select::Function::Count, ""),
            clause.clone(),
            Vec::new(),
            Some(10),
            false,
        );
        request.having.push(clause);

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("multiple HAVING clauses")
                        && message.contains("not yet implemented"),
                    "expected the multi-clause rejection, got: {message}"
                );
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// `GROUP BY restaurantId, guests HAVING …` — an **unpinned**
    /// two-field grouping — is still rejected: the routing layer sends
    /// any grouped single-clause having down the having path (it owns
    /// *where* the request goes, not the grammar), and drive's mode
    /// detection rejects the compound grouping, steering the caller to
    /// the served pinned form (`WHERE <leading> = X GROUP BY
    /// <trailing>` — exercised end to end in
    /// [`pinned_prefix_having_is_served_end_to_end`]).
    #[test]
    fn compound_group_by_is_rejected_on_the_having_path() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let mut request = having_request(
            &contract,
            "visit",
            select(v1_select::Function::Count, ""),
            hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(2),
            ),
            Vec::new(),
            Some(10),
            false,
        );
        request.group_by = vec![GROUP_PROPERTY.to_string(), "guests".to_string()];

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::InvalidParameter(message)) => {
                assert!(
                    message.contains("exactly one `group_by` property")
                        && message.contains("pin every leading index property"),
                    "the rejection must steer to the pinned-prefix form, got: {message}"
                );
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    /// Shared with rs-drive's `pinned_prefix` suites — the compound
    /// ranked index `[identityId, class]` with the Avg axis on `grade`.
    const GRADES_COMPOUND_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/grades/grades-compound-ranked-contract.json";

    fn register_grades_compound(
        platform: &Platform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> dpp::prelude::DataContract {
        let contract =
            json_document_to_contract(GRADES_COMPOUND_CONTRACT_PATH, false, platform_version)
                .expect("expected to parse the compound ranked grades contract");
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    fn insert_grade_docs(
        platform: &Platform<MockCoreRPCLike>,
        contract: &dpp::prelude::DataContract,
        first_seed: u64,
        rows: &[([u8; 32], &str, i64)],
        platform_version: &PlatformVersion,
    ) {
        let document_type = contract
            .document_type_for_name("grade")
            .expect("grade doctype exists");
        for (i, (identity, class, grade)) in rows.iter().enumerate() {
            let mut document: Document = document_type
                .random_document(Some(first_seed + i as u64), platform_version)
                .expect("random document");
            let mut properties = BTreeMap::new();
            properties.insert("identityId".to_string(), Value::Identifier(*identity));
            properties.insert("class".to_string(), Value::Text(class.to_string()));
            properties.insert("grade".to_string(), Value::I64(*grade));
            document.set_properties(properties);
            store_document(
                platform,
                contract,
                document_type,
                &document,
                platform_version,
            );
        }
    }

    /// The `IN`-pinned form end to end on the wire: `WHERE identityId
    /// IN [X, Y] GROUP BY class HAVING AVG(grade) > 80 LIMIT 10` fans
    /// out across both identities' secondaries and answers one merged
    /// `ResultData.ranked` page whose entries carry `in_key`; the
    /// proved variant returns the unified branched `PathQuery` envelope as its Proof
    /// payload. Merge/proof semantics are pinned in rs-drive's suites;
    /// this pins the wire encoding, routing, and `in_key` mapping.
    #[test]
    fn in_pinned_having_is_served_end_to_end() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_grades_compound(&platform, version);
        let identity_x = [1u8; 32];
        let identity_y = [2u8; 32];
        insert_grade_docs(
            &platform,
            &contract,
            21_000,
            &[
                (identity_x, "art", 90),
                (identity_x, "math", 60),
                (identity_y, "science", 95),
            ],
            version,
        );

        let mut request = having_request(
            &contract,
            "grade",
            select(v1_select::Function::Avg, "grade"),
            hc(
                having_aggregate::Function::Avg,
                "grade",
                having_clause::Operator::GreaterThan,
                Value::U64(80),
            ),
            Vec::new(),
            Some(10),
            false,
        );
        request.group_by = vec!["class".to_string()];
        request.where_clauses = vec![wc(
            "identityId",
            ProtoWhereOperator::In,
            Value::Array(vec![
                Value::Bytes(identity_y.to_vec()),
                Value::Bytes(identity_x.to_vec()),
            ]),
        )];

        let page = ranked_page(&platform, &state, request.clone(), version);
        assert_eq!(
            group_keys(&page.entries),
            vec!["art", "science"],
            "merged ascending: X's art (90) then Y's science (95); X's math \
             (60) misses the bound"
        );
        assert_eq!(
            page.entries
                .iter()
                .map(|e| e.in_key.clone())
                .collect::<Vec<_>>(),
            vec![Some(identity_x.to_vec()), Some(identity_y.to_vec())],
            "merged entries carry their branch's in_key on the wire"
        );

        // The proved variant answers with a Proof payload (the unified
        // branched `PathQuery` envelope — verified client-side, pinned
        // in rs-drive's tamper suite).
        request.prove = true;
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query should succeed");
        assert!(result.errors.is_empty(), "got {:?}", result.errors);
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                ..
            }) => assert!(!proof.grovedb_proof.is_empty()),
            other => panic!("expected a Proof response, got {:?}", other),
        }
    }

    /// The pinned-prefix form end to end on the wire: `WHERE identityId
    /// = X GROUP BY class HAVING AVG(grade) > 80 LIMIT 10` routes to
    /// the same having executor, descends to X's terminal `class` tree,
    /// and answers with `ResultData.ranked` (`skipped` unset) — with
    /// per-prefix isolation visible in the entries. Value-level
    /// behaviour (bounds, proofs, tamper) is pinned in rs-drive's
    /// `pinned_prefix` suite; this pins the wire encoding and routing.
    #[test]
    fn pinned_prefix_having_is_served_end_to_end() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_grades_compound(&platform, version);
        let identity_x = [1u8; 32];
        let identity_y = [2u8; 32];
        insert_grade_docs(
            &platform,
            &contract,
            20_000,
            &[
                (identity_x, "math", 80),
                (identity_x, "math", 80),
                (identity_x, "english", 80),
                (identity_x, "english", 81),
                (identity_x, "art", 85),
                (identity_x, "art", 95),
                // Y's science (avg 95) would qualify for X's bound too
                // if the prefixes shared a secondary.
                (identity_y, "science", 95),
                (identity_y, "science", 95),
            ],
            version,
        );

        let mut request = having_request(
            &contract,
            "grade",
            select(v1_select::Function::Avg, "grade"),
            hc(
                having_aggregate::Function::Avg,
                "grade",
                having_clause::Operator::GreaterThan,
                Value::U64(80),
            ),
            Vec::new(),
            Some(10),
            false,
        );
        request.group_by = vec!["class".to_string()];
        request.where_clauses = vec![wc(
            "identityId",
            ProtoWhereOperator::Equal,
            Value::Bytes(identity_x.to_vec()),
        )];

        let page = ranked_page(&platform, &state, request.clone(), version);
        assert_eq!(
            page.skipped, None,
            "a having-range page must leave the rank-based `skipped` field unset"
        );
        assert_eq!(
            group_keys(&page.entries),
            vec!["english", "art"],
            "ascending average order over X's own classes: english (80.5) before art (90); \
             math sits exactly at the threshold and Y's science must not leak in"
        );

        // The proved variant answers with a Proof payload.
        request.prove = true;
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(
            result.errors.is_empty(),
            "expected no validation errors, got {:?}",
            result.errors
        );
        match result.data {
            Some(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(_)),
                metadata: Some(_),
            }) => {}
            other => panic!("expected a Proof result, got {:?}", other),
        }
    }

    /// The pinned-prefix shape is still a HAVING request, and protocol
    /// version 13's query table (v0 helper) has no having path: the
    /// same request a v14 node serves is refused with the blanket
    /// rejection before any contract fetch.
    #[test]
    fn pinned_prefix_having_still_rejected_at_protocol_version_13() {
        let (platform, state, version) =
            setup_platform(None, Network::Testnet, Some(PROTOCOL_VERSION_V13));

        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "grade".to_string(),
            where_clauses: vec![wc(
                "identityId",
                ProtoWhereOperator::Equal,
                Value::Bytes(vec![1u8; 32]),
            )],
            order_by: Vec::new(),
            limit: Some(10),
            start: None,
            prove: false,
            selects: select(v1_select::Function::Avg, "grade"),
            group_by: vec!["class".to_string()],
            having: vec![hc(
                having_aggregate::Function::Avg,
                "grade",
                having_clause::Operator::GreaterThan,
                Value::U64(80),
            )],
            offset: None,
        };

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("HAVING clause") && message.contains("not yet implemented"),
                    "expected v13's blanket rejection, got: {message}"
                );
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// `OFFSET` stays ranked-only, and the having-range route gets its
    /// own rejection: the legacy message recommends `start_after` /
    /// `start_at`, which this surface also rejects, so the having
    /// message explains continuation-by-bound instead of pointing at
    /// an unsupported cursor.
    #[test]
    fn offset_is_rejected_on_the_having_path() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let mut request = having_request(
            &contract,
            "visit",
            select(v1_select::Function::Count, ""),
            hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(2),
            ),
            Vec::new(),
            Some(10),
            false,
        );
        request.offset = Some(1);

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("OFFSET on a having-range query"),
                    "expected the having-specific offset message, got: {message}"
                );
                assert!(
                    message.contains("tighten the `having` bound"),
                    "the message must explain continuation-by-bound, got: {message}"
                );
                assert!(
                    !message.contains("start_after"),
                    "the message must not recommend cursors this surface rejects, got: {message}"
                );
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// A bound on an axis the index does not declare surfaces as a
    /// query error naming the missing contract keyword. The `review`
    /// doctype's index is `rankedAverageable` only.
    #[test]
    fn a_bound_on_an_undeclared_axis_names_the_missing_keyword() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        let request = having_request(
            &contract,
            "review",
            select(v1_select::Function::Count, ""),
            hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(2),
            ),
            Vec::new(),
            Some(10),
            false,
        );

        let error = ranked_error(&platform, &state, request, version);
        assert!(
            format!("{error}").contains("rankedCountable"),
            "the rejection must name the missing keyword, got: {error}"
        );
    }

    /// Non-contiguous operators (`!=`, `IN`) reach drive and are
    /// refused there with a message explaining the contiguity
    /// requirement — as a query error, never an internal one.
    #[test]
    fn non_contiguous_operators_surface_as_query_errors() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let contract = register_restaurants(&platform, version);

        for operator in [
            having_clause::Operator::NotEqual,
            having_clause::Operator::In,
        ] {
            let request = having_request(
                &contract,
                "visit",
                select(v1_select::Function::Count, ""),
                hc(
                    having_aggregate::Function::Count,
                    "",
                    operator,
                    Value::U64(2),
                ),
                Vec::new(),
                Some(10),
                false,
            );
            match ranked_error(&platform, &state, request, version) {
                QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                    assert!(
                        message.contains("contiguous"),
                        "expected the contiguity rejection for {operator:?}, got: {message}"
                    );
                }
                other => panic!("expected Unsupported for {operator:?}, got {other:?}"),
            }
        }
    }

    /// Protocol version 13's query table (v0 helper) has no having
    /// path: the same request a v14 node answers is refused with the
    /// blanket rejection. The routing gate fires before any contract
    /// fetch, so no ranked contract is needed (v13's meta-schema could
    /// not register one anyway).
    #[test]
    fn protocol_version_13_still_rejects_having() {
        let (platform, state, version) =
            setup_platform(None, Network::Testnet, Some(PROTOCOL_VERSION_V13));

        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "visit".to_string(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: Some(10),
            start: None,
            prove: false,
            selects: select(v1_select::Function::Count, ""),
            group_by: vec![GROUP_PROPERTY.to_string()],
            having: vec![hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(2),
            )],
            offset: None,
        };

        match ranked_error(&platform, &state, request, version) {
            QueryError::Query(QuerySyntaxError::Unsupported(message)) => {
                assert!(
                    message.contains("HAVING clause") && message.contains("not yet implemented"),
                    "expected v13's blanket rejection, got: {message}"
                );
            }
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }

    /// The routing label: a grouped aggregate with one having clause
    /// routes to `having_range`, and without the group_by it stays on
    /// the blanket-rejection path.
    #[test]
    fn routing_picks_having_range_only_for_grouped_single_clause_having() {
        let clause = hc(
            having_aggregate::Function::Count,
            "",
            having_clause::Operator::GreaterThan,
            Value::U64(2),
        );

        let grouped = GetDocumentsRequestV1 {
            selects: select_count_star(),
            group_by: vec![GROUP_PROPERTY.to_string()],
            having: vec![clause.clone()],
            limit: Some(10),
            ..empty_v1_request()
        };
        let label = validate_and_route_for_tests(&grouped, &[], PlatformVersion::latest())
            .expect("a grouped single-clause having is a supported shape");
        assert_eq!(label, "having_range");

        // No group_by → the v0 blanket rejection still owns it.
        let ungrouped = GetDocumentsRequestV1 {
            group_by: Vec::new(),
            ..grouped
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&ungrouped, &[], PlatformVersion::latest()),
            "HAVING clause",
        );
    }
}

mod having_trust_boundary {
    //! The client trust boundary, exercised from the server side: a
    //! grovedb-valid having proof is only an authenticated platform
    //! result once drive-proof-verifier's [`verify_having_range_proof`]
    //! wrapper binds its reconstructed root hash to the quorum-signed
    //! app hash. These tests generate a real AVG having proof from a
    //! real Drive, sign the canonical tenderdash precommit with a test
    //! quorum key, and run the client wrapper end to end: the correctly
    //! signed root verifies, and a commit over a different app hash,
    //! tampered response metadata, or a wrong quorum key each fail —
    //! so omitting or miswiring `verify_tenderdash_proof` turns a test
    //! red. The suite lives here rather than in drive-proof-verifier
    //! because generating proofs needs drive's server feature, which
    //! the client crate must not enable even as a dev-dependency.

    use crate::platform_types::platform::Platform;
    use crate::query::tests::{setup_platform, store_data_contract};
    use crate::rpc::core::MockCoreRPCLike;
    use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
    use dpp::block::block_info::BlockInfo;
    use dpp::bls_signatures::{Bls12381G2Impl, SecretKey, SignatureSchemes};
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::TokenConfiguration;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::platform_value::Value;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dpp::tests::json_document::json_document_to_contract;
    use dpp::version::PlatformVersion;
    use drive::drive::Drive;
    use drive::query::drive_document_having_query::drive_dispatcher::{
        DocumentHavingRequest, DocumentHavingResponse,
    };
    use drive::query::drive_document_having_query::mode_detection::detect_having_mode;
    use drive::query::drive_document_having_query::resolve_having_query_for_mode;
    use drive::query::having::{
        HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
    };
    use drive::query::projection::SelectProjection;
    use drive::query::{DriveDocumentHavingQuery, RankedPaginationInputs};
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use drive_proof_verifier::{
        verify_having_range_proof, ContextProvider, ContextProviderError,
        Error as ProofVerifierError,
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tenderdash_abci::proto::types::{CanonicalVote, SignedMsgType, StateId};
    use tenderdash_abci::signatures::{Hashable, Signable};

    /// Shared with rs-drive's having suite — same cross-crate path
    /// convention as `RESTAURANTS_CONTRACT_PATH` above.
    const GRADES_RANKED_CONTRACT_PATH: &str =
        "../rs-drive/tests/supporting_files/contract/grades/grades-ranked-contract.json";

    const CHAIN_ID: &str = "test-having-chain";
    const HEIGHT: u64 = 4242;
    const ROUND: u32 = 0;
    const QUORUM_TYPE: u32 = 1; // LLMQ_50_60
    const CORE_LOCKED_HEIGHT: u32 = 1200;
    const TIME_MS: u64 = 1_755_000_000_000;

    /// Provider that knows exactly one quorum key — the test one.
    ///
    /// This and the two signing helpers below are `pub(super)` so the
    /// sibling [`super::time_range_proof_verification`] suite signs its
    /// commits through the same canonical construction; a second copy of
    /// the tenderdash harness could drift from this one and quietly stop
    /// testing the binding.
    pub(super) struct TestQuorumProvider {
        pub(super) pubkey: [u8; 48],
    }

    impl ContextProvider for TestQuorumProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok(self.pubkey)
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    /// A deterministic, valid BLS scalar — no RNG dependency.
    pub(super) fn quorum_secret_key() -> SecretKey<Bls12381G2Impl> {
        let mut bytes = [0u8; 32];
        bytes[31] = 42;
        SecretKey::<Bls12381G2Impl>::from_be_bytes(&bytes)
            .into_option()
            .expect("a small nonzero scalar is a valid secret key")
    }

    fn register_grades(
        platform: &Platform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let contract =
            json_document_to_contract(GRADES_RANKED_CONTRACT_PATH, false, platform_version)
                .expect("expected to parse the ranked grades contract");
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    /// A few grade documents so the axis secondary has content to
    /// prove over: identity `[1; 32]` averages 75, identity `[2; 32]`
    /// averages 90.
    fn insert_grades(platform: &Platform<MockCoreRPCLike>, contract: &DataContract) {
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name("grade")
            .expect("grade doctype exists");
        let rows: [([u8; 32], i64); 4] = [
            ([1u8; 32], 70),
            ([1u8; 32], 80),
            ([2u8; 32], 85),
            ([2u8; 32], 95),
        ];
        for (i, (identity, grade)) in rows.iter().enumerate() {
            let mut doc: Document = document_type
                .random_document(Some(9000 + i as u64), pv)
                .expect("random document");
            let mut props = BTreeMap::new();
            props.insert("identityId".to_string(), Value::Identifier(*identity));
            props.insert("grade".to_string(), Value::I64(*grade));
            doc.set_properties(props);
            platform
                .drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&doc, None)),
                            owner_id: None,
                        },
                        contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert a grade document");
        }
    }

    /// `AVG(grade) > 80 LIMIT 10` — matches identity `[2; 32]`
    /// (average 90) and excludes identity `[1; 32]` (average 75).
    fn having_clause() -> HavingClause {
        HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Avg,
                field: "grade".to_string(),
            },
            operator: HavingOperator::GreaterThan,
            right: HavingRightOperand::Value(Value::U64(80)),
        }
    }

    fn client_side_query(contract: &DataContract) -> DriveDocumentHavingQuery<'_> {
        let group_by = vec!["identityId".to_string()];
        let having = vec![having_clause()];
        let mode = detect_having_mode(
            &SelectProjection::avg("grade"),
            &group_by,
            &having,
            &[],
            &[],
            RankedPaginationInputs {
                limit: Some(10),
                offset: None,
                has_start_at: false,
            },
            platform_version(),
        )
        .expect("the case is well-formed");
        resolve_having_query_for_mode(
            contract.id_ref().to_buffer(),
            contract
                .document_type_for_name("grade")
                .expect("grade doctype exists"),
            "grade".to_string(),
            contract
                .document_types()
                .get("grade")
                .expect("grade doctype exists")
                .indexes(),
            &mode,
            PlatformVersion::latest(),
        )
        .expect("the fixture declares the avg axis")
    }

    /// Prove the having request against the live Drive and return
    /// `(grovedb proof bytes, live root hash)`.
    fn prove(drive: &Drive, contract: &DataContract) -> (Vec<u8>, [u8; 32]) {
        let group_by = vec!["identityId".to_string()];
        let having = vec![having_clause()];
        let response = drive
            .execute_document_having_request(
                DocumentHavingRequest {
                    contract,
                    document_type: contract
                        .document_type_for_name("grade")
                        .expect("grade doctype exists"),
                    group_by: &group_by,
                    select: SelectProjection::avg("grade"),
                    having: &having,
                    order_by: &[],
                    where_clauses: &[],
                    resolved_time_ranges: &[],
                    limit: Some(10),
                    offset: None,
                    has_start_at: false,
                    prove: true,
                },
                None,
                platform_version(),
            )
            .expect("the prove request must execute");
        let proof_bytes = match response {
            DocumentHavingResponse::Proof(proof) => proof,
            DocumentHavingResponse::Entries(_) => panic!("expected a proof, got entries"),
        };
        let root_hash = drive
            .grove
            .root_hash(None, &platform_version().drive.grove_version)
            .unwrap()
            .expect("root hash must be readable");
        (proof_bytes, root_hash)
    }

    fn metadata() -> ResponseMetadata {
        ResponseMetadata {
            height: HEIGHT,
            core_chain_locked_height: CORE_LOCKED_HEIGHT,
            epoch: 0,
            time_ms: TIME_MS,
            protocol_version: platform_version().protocol_version,
            chain_id: CHAIN_ID.to_string(),
        }
    }

    /// Sign a tenderdash precommit whose state id carries `app_hash` —
    /// the same canonical construction `verify_tenderdash_proof`
    /// rebuilds on the verify side.
    pub(super) fn signed_proof(
        grovedb_proof: Vec<u8>,
        app_hash: &[u8; 32],
        mtd: &ResponseMetadata,
        secret_key: &SecretKey<Bls12381G2Impl>,
        quorum_hash: [u8; 32],
    ) -> Proof {
        let block_id_hash = [7u8; 32].to_vec();
        let state_id = StateId {
            app_version: mtd.protocol_version as u64,
            core_chain_locked_height: mtd.core_chain_locked_height,
            time: mtd.time_ms,
            app_hash: app_hash.to_vec(),
            height: mtd.height,
        };
        let state_id_hash = state_id
            .calculate_msg_hash(&mtd.chain_id, mtd.height as i64, ROUND as i32)
            .expect("state id hash");
        let commit = CanonicalVote {
            r#type: SignedMsgType::Precommit.into(),
            block_id: block_id_hash.clone(),
            chain_id: mtd.chain_id.clone(),
            height: mtd.height as i64,
            round: ROUND as i64,
            state_id: state_id_hash,
        };
        let sign_digest = commit
            .calculate_sign_hash(
                &mtd.chain_id,
                QUORUM_TYPE.try_into().expect("valid quorum type"),
                &quorum_hash,
                mtd.height as i64,
                ROUND as i32,
            )
            .expect("sign digest");
        let signature = secret_key
            .sign(SignatureSchemes::Basic, &sign_digest)
            .expect("signing with a valid key succeeds")
            .as_raw_value()
            .to_compressed()
            .to_vec();
        Proof {
            grovedb_proof,
            quorum_hash: quorum_hash.to_vec(),
            signature,
            round: ROUND,
            block_id_hash,
            quorum_type: QUORUM_TYPE,
        }
    }

    /// The full composition succeeds end to end: merk verification
    /// reconstructs the root, the tenderdash commit over that root
    /// verifies against the quorum key, and the verified entries are
    /// exactly the matching groups.
    #[test]
    fn a_correctly_signed_root_verifies_and_returns_the_matches() {
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);
        let contract = register_grades(&platform, platform_version());
        insert_grades(&platform, &contract);
        let (grovedb_proof, root_hash) = prove(&platform.drive, &contract);
        let secret_key = quorum_secret_key();
        let quorum_hash = [3u8; 32];
        let mtd = metadata();
        let proof = signed_proof(grovedb_proof, &root_hash, &mtd, &secret_key, quorum_hash);
        let provider = TestQuorumProvider {
            pubkey: secret_key.public_key().0.to_compressed(),
        };

        let query = client_side_query(&contract);
        let (verified_root, entries) =
            verify_having_range_proof(&query, &proof, &mtd, platform_version(), &provider)
                .expect("a correctly signed root must verify");

        assert_eq!(verified_root, root_hash);
        assert_eq!(
            entries.iter().map(|e| e.key.clone()).collect::<Vec<_>>(),
            vec![[2u8; 32].to_vec()],
            "only the identity averaging 90 clears the > 80 bound"
        );
    }

    /// A commit signed over a *different* app hash must not verify:
    /// the node's grovedb proof reconstructs the true root, and the
    /// tenderdash binding is what catches the mismatch.
    #[test]
    fn a_commit_over_a_different_app_hash_is_rejected() {
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);
        let contract = register_grades(&platform, platform_version());
        insert_grades(&platform, &contract);
        let (grovedb_proof, _root_hash) = prove(&platform.drive, &contract);
        let secret_key = quorum_secret_key();
        let quorum_hash = [3u8; 32];
        let mtd = metadata();
        let wrong_app_hash = [0xAA; 32];
        let proof = signed_proof(
            grovedb_proof,
            &wrong_app_hash,
            &mtd,
            &secret_key,
            quorum_hash,
        );
        let provider = TestQuorumProvider {
            pubkey: secret_key.public_key().0.to_compressed(),
        };

        let query = client_side_query(&contract);
        let error = verify_having_range_proof(&query, &proof, &mtd, platform_version(), &provider)
            .expect_err("a commit over a different app hash must be rejected");
        assert!(
            matches!(error, ProofVerifierError::InvalidSignature { .. }),
            "the rejection must be the signature binding, got: {error:?}"
        );
    }

    /// Tampered response metadata changes the canonical state id, so a
    /// signature over the honest metadata stops verifying.
    #[test]
    fn tampered_metadata_is_rejected() {
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);
        let contract = register_grades(&platform, platform_version());
        insert_grades(&platform, &contract);
        let (grovedb_proof, root_hash) = prove(&platform.drive, &contract);
        let secret_key = quorum_secret_key();
        let quorum_hash = [3u8; 32];
        let mtd = metadata();
        let proof = signed_proof(grovedb_proof, &root_hash, &mtd, &secret_key, quorum_hash);
        let provider = TestQuorumProvider {
            pubkey: secret_key.public_key().0.to_compressed(),
        };

        let mut tampered = mtd;
        tampered.height += 1;

        let query = client_side_query(&contract);
        let error =
            verify_having_range_proof(&query, &proof, &tampered, platform_version(), &provider)
                .expect_err("tampered metadata must be rejected");
        assert!(
            matches!(error, ProofVerifierError::InvalidSignature { .. }),
            "the rejection must be the signature binding, got: {error:?}"
        );
    }

    /// A provider vending a different quorum key models a signer
    /// outside the expected quorum: the commit must not verify.
    #[test]
    fn a_wrong_quorum_key_is_rejected() {
        let (platform, _state, _version) = setup_platform(None, Network::Testnet, None);
        let contract = register_grades(&platform, platform_version());
        insert_grades(&platform, &contract);
        let (grovedb_proof, root_hash) = prove(&platform.drive, &contract);
        let secret_key = quorum_secret_key();
        let quorum_hash = [3u8; 32];
        let mtd = metadata();
        let proof = signed_proof(grovedb_proof, &root_hash, &mtd, &secret_key, quorum_hash);

        let mut other_bytes = [0u8; 32];
        other_bytes[31] = 43;
        let other_key = SecretKey::<Bls12381G2Impl>::from_be_bytes(&other_bytes)
            .into_option()
            .expect("valid scalar");
        let provider = TestQuorumProvider {
            pubkey: other_key.public_key().0.to_compressed(),
        };

        let query = client_side_query(&contract);
        let error = verify_having_range_proof(&query, &proof, &mtd, platform_version(), &provider)
            .expect_err("a commit signed outside the expected quorum must be rejected");
        assert!(
            matches!(error, ProofVerifierError::InvalidSignature { .. }),
            "the rejection must be the signature binding, got: {error:?}"
        );
    }
}

mod time_range_proof_verification {
    //! The whole time-range reconstruction sequence, run end to end
    //! against a populated Drive: the handler resolves `IN_TIME_RANGE`
    //! from committed block time, proves the resulting bucket query,
    //! and the client re-derives the *identical* bucket from the
    //! quorum-signed response metadata `time_ms`, re-runs the
    //! provenance guard, re-picks the bucketed index and verifies the
    //! proof over the GroveDB path that selection produces. Every link
    //! in that chain is load-bearing: a resolution reading anything
    //! but the signed time, a picker admitting the plain index, or a
    //! guard accepting a shape the resolver never built would each
    //! turn a wrong answer into a *proven* wrong answer.
    //!
    //! The headline assertion is the overlapping-window one. With
    //! `range = 6h, step = 2h` a document is written under three
    //! bucket keys at once, so a count that walked buckets rather
    //! than addressing exactly one would return three times the
    //! truth — and would verify, because the proof would be an
    //! honest proof of the wrong query.
    //!
    //! Split, same as [`super::having_trust_boundary`]: proof
    //! end-to-end lives here because generating proofs needs drive's
    //! `server` feature and a populated platform; the client-surface
    //! half (resolution from metadata time, provenance bookkeeping,
    //! ordering) is offline-tested in rs-sdk, which builds drive with
    //! `verify` only.

    use super::having_trust_boundary::{quorum_secret_key, signed_proof, TestQuorumProvider};
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dapi_grpc::platform::v0::get_documents_response::Version as ResponseVersion;
    use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value::platform_value;
    use dpp::prelude::DataContract;
    use drive::drive::Drive;
    use drive::query::{
        DriveDocumentCountQuery, DriveDocumentQuery, TimeRangeSelector, WhereClause,
    };
    use drive_proof_verifier::types::Documents;
    use drive_proof_verifier::{
        verify_point_lookup_count_proof, Error as ProofVerifierError, FromProof, SplitCountEntry,
    };
    use std::collections::BTreeMap;

    const DOCUMENT_TYPE: &str = "post";
    const CREATED_AT: &str = "$createdAt";
    const BUCKETED_INDEX: &str = "trending";
    const MINUTE_MS: u64 = 60_000;
    const HOUR_MS: u64 = 3_600_000;

    /// A bucket start on the contract's grid: `origin` is 0 and the step is
    /// two hours, and this is an exact multiple of two hours. Every timestamp
    /// below is expressed relative to it.
    const NEWEST_BUCKET_START_MS: u64 = 1_755_000_000_000;
    /// Committed block time, one hour into the newest bucket — so the newest
    /// active range is `NEWEST_BUCKET_START_MS` and there is a full hour of
    /// bucket in which documents can sit.
    const BLOCK_TIME_MS: u64 = NEWEST_BUCKET_START_MS + HOUR_MS;
    const BLOCK_HEIGHT: u64 = 100;
    const BLOCK_CORE_HEIGHT: u32 = 42;
    const QUORUM_HASH: [u8; 32] = [3u8; 32];

    /// `($createdAt, hashtag)` for the four posts, chosen so the newest
    /// bucket contains: two `#ibiza` posts that *also* live in the two older
    /// overlapping buckets (the double-count regression), and one `#berlin`
    /// post (so the second index property has to do work). The third
    /// `#ibiza` post starts one hour before the newest bucket, so it belongs
    /// to the three *older* buckets and to none of the newest — the negative
    /// control that pins the query to a single bucket rather than to "recent
    /// enough".
    const POSTS: [(u64, &str); 4] = [
        (NEWEST_BUCKET_START_MS + 10 * MINUTE_MS, "ibiza"),
        (NEWEST_BUCKET_START_MS + 30 * MINUTE_MS, "ibiza"),
        (NEWEST_BUCKET_START_MS - HOUR_MS, "ibiza"),
        (NEWEST_BUCKET_START_MS + 15 * MINUTE_MS, "berlin"),
    ];

    /// A `countable` bucketed index over `(timeRange($createdAt), hashtag)`
    /// with a six-hour window sliding every two hours — overlap factor 3 —
    /// next to a plain index covering the same two fields the other way
    /// round. The plain one exists so index selection has something wrong to
    /// pick: it covers the identical clause field set, and only the
    /// resolution provenance keeps the query off it.
    fn register_trending_contract(
        platform: &Platform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let factory = DataContractFactory::new(platform_version.protocol_version)
            .expect("expected a factory");
        let schemas = platform_value!({
            DOCUMENT_TYPE: {
                "type": "object",
                "properties": {
                    "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
                },
                "indices": [
                    {
                        "name": BUCKETED_INDEX,
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "countable": true,
                        "timeRange": { "on": "$createdAt", "range": 21_600u64, "step": 7_200u64 },
                    },
                    {
                        "name": "byHashtag",
                        "properties": [{ "hashtag": "asc" }, { "$createdAt": "asc" }],
                        "countable": true,
                    },
                ],
                "required": ["$createdAt", "hashtag"],
                "additionalProperties": false,
            }
        });
        let contract = factory
            .create_with_value_config(Identifier::new([7u8; 32]), 0, schemas, None, None)
            .expect("the trending contract is well-formed")
            .data_contract_owned();
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    fn insert_posts(
        platform: &Platform<MockCoreRPCLike>,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        POSTS
            .iter()
            .enumerate()
            .map(|(i, (created_at_ms, hashtag))| {
                let mut document: Document = document_type
                    .random_document(Some(7_000 + i as u64), platform_version)
                    .expect("random document");
                document.set_properties(BTreeMap::from([(
                    "hashtag".to_string(),
                    Value::Text(hashtag.to_string()),
                )]));
                document.set_created_at(Some(*created_at_ms));
                store_document(
                    platform,
                    contract,
                    document_type,
                    &document,
                    platform_version,
                );
                document
            })
            .collect()
    }

    fn root_hash(drive: &Drive, platform_version: &PlatformVersion) -> [u8; 32] {
        drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash must be readable")
    }

    /// A state whose last committed block carries [`BLOCK_TIME_MS`] — the
    /// handler resolves `IN_TIME_RANGE` from it and stamps it into the
    /// response metadata, which is what makes the client's re-derivation
    /// land on the same bucket.
    fn state_with_committed_block_time(
        base: &PlatformState,
        drive: &Drive,
        platform_version: &PlatformVersion,
    ) -> PlatformState {
        let mut state = base.clone();
        state.set_last_committed_block_info(Some(
            ExtendedBlockInfoV0 {
                basic_info: BlockInfo {
                    time_ms: BLOCK_TIME_MS,
                    height: BLOCK_HEIGHT,
                    core_height: BLOCK_CORE_HEIGHT,
                    epoch: Default::default(),
                },
                app_hash: root_hash(drive, platform_version),
                quorum_hash: [0u8; 32],
                block_id_hash: [0u8; 32],
                proposer_pro_tx_hash: [0u8; 32],
                signature: [0u8; 96],
                round: 0,
            }
            .into(),
        ));
        state
    }

    /// Contract registered, posts inserted, and a platform state whose
    /// committed block time is [`BLOCK_TIME_MS`].
    fn setup_trending(
        platform: &TempPlatform<MockCoreRPCLike>,
        base_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> (DataContract, Vec<Document>, PlatformState) {
        assert!(
            platform_version.protocol_version >= 14,
            "time-range indexes require protocol version 14 or later"
        );
        let contract = register_trending_contract(platform, platform_version);
        let documents = insert_posts(platform, &contract, platform_version);
        let state = state_with_committed_block_time(base_state, &platform.drive, platform_version);
        (contract, documents, state)
    }

    /// `IN_TIME_RANGE($createdAt, "newest") AND hashtag = <hashtag>` — the
    /// selector rides the wire as an operator, never as a resolved value, so
    /// the bucket in the proof can only have come from the node's committed
    /// block time.
    fn trending_where_clauses(hashtag: &str) -> Vec<ProtoWhereClause> {
        vec![
            wc(
                "hashtag",
                ProtoWhereOperator::Equal,
                Value::Text(hashtag.to_string()),
            ),
            wc(
                CREATED_AT,
                ProtoWhereOperator::InTimeRange,
                Value::Text(TimeRangeSelector::Newest.as_str().to_string()),
            ),
        ]
    }

    fn trending_request(
        contract_id: Vec<u8>,
        hashtag: &str,
        selects: Vec<V1Select>,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: contract_id,
            document_type: DOCUMENT_TYPE.to_string(),
            where_clauses: trending_where_clauses(hashtag),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: true,
            selects,
            group_by: Vec::new(),
            having: Vec::new(),
            offset: None,
        }
    }

    /// Run the request through the real v1 handler and re-sign the proof it
    /// produced: the test platform never signs a commit, so the tenderdash
    /// binding is supplied here over the *live* root hash and the handler's
    /// own response metadata. Tampering with either afterwards is what the
    /// negative case does.
    fn prove_and_sign(
        platform: &TempPlatform<MockCoreRPCLike>,
        state: &PlatformState,
        request: GetDocumentsRequestV1,
        platform_version: &PlatformVersion,
    ) -> (Proof, ResponseMetadata, TestQuorumProvider) {
        let result = platform
            .query_documents_v1(request, state, platform_version)
            .expect("query call should not error at the transport layer");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let response = result.data.expect("data");
        let proof = match response.result {
            Some(get_documents_response_v1::Result::Proof(proof)) => proof,
            other => panic!("expected a proof, got {:?}", other),
        };
        let mtd = response
            .metadata
            .expect("the handler stamps response metadata");
        assert_eq!(
            mtd.time_ms, BLOCK_TIME_MS,
            "the metadata time the client resolves from must be the committed \
             block time the server resolved from"
        );

        let secret_key = quorum_secret_key();
        let signed = signed_proof(
            proof.grovedb_proof,
            &root_hash(&platform.drive, platform_version),
            &mtd,
            &secret_key,
            QUORUM_HASH,
        );
        let provider = TestQuorumProvider {
            pubkey: secret_key.public_key().0.to_compressed(),
        };
        (signed, mtd, provider)
    }

    /// Re-sign the handler's proof over *altered* metadata with the fixture
    /// quorum key. The signature then verifies, so a rejection can only come
    /// from the proof path itself: the verifier resolves the selector from
    /// the altered time into the NEXT bucket, reconstructs that bucket's
    /// path query, and the GroveDB proof over the original bucket cannot
    /// satisfy it. Without the re-signing, the negative tests could pass at
    /// signature verification and never reach that property.
    fn resign_over(
        platform: &TempPlatform<MockCoreRPCLike>,
        proof: &Proof,
        mtd: &ResponseMetadata,
        platform_version: &PlatformVersion,
    ) -> Proof {
        signed_proof(
            proof.grovedb_proof.clone(),
            &root_hash(&platform.drive, platform_version),
            mtd,
            &quorum_secret_key(),
            QUORUM_HASH,
        )
    }

    /// The bucket start the transform puts `time_ms` in, computed straight
    /// off the contract's declared window rather than off the constants
    /// above — so a fixture edit that moves the grid cannot leave the
    /// expectation behind.
    fn expected_newest_bucket(contract: &DataContract, time_ms: u64) -> u64 {
        contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists")
            .indexes()
            .get(BUCKETED_INDEX)
            .expect("the bucketed index survives contract registration")
            .time_range
            .as_ref()
            .expect("the bucketed index carries its transform")
            .newest_active_start(time_ms)
            .expect("the block time is inside an active range")
    }

    /// The client's reconstruction sequence, step for step, exactly as the
    /// SDK's count helper performs it: resolve the selector from the signed
    /// metadata time, record the provenance, re-run the shape guard, pick
    /// the index that provenance admits, rebuild the prover's count query.
    ///
    /// Returns the resolved bucket start alongside the query so callers can
    /// assert on what the metadata time resolved to.
    fn client_count_query<'a>(
        contract: &'a DataContract,
        document_type: &'a DocumentTypeRef<'a>,
        hashtag: &str,
        time_ms: u64,
    ) -> (DriveDocumentCountQuery<'a>, u64) {
        let mut where_clauses = vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(hashtag.to_string()),
        }];
        let (clause, resolution) = resolve_time_range_bucket_clause(
            CREATED_AT,
            TimeRangeSelector::Newest,
            None,
            *document_type,
            time_ms,
        )
        .expect("the metadata time falls inside an active range");
        let bucket_start = clause
            .value
            .to_integer::<u64>()
            .expect("a resolved bucket start is a millisecond timestamp");
        where_clauses.push(clause);
        let resolutions = vec![resolution];

        validate_resolved_time_range_clause_shapes(&where_clauses, &resolutions)
            .expect("resolution produces exactly the one equality the guard admits");

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
            &resolutions,
        )
        .expect("the bucketed index covers the resolved clause set");
        assert_eq!(
            index.name, BUCKETED_INDEX,
            "resolution provenance must pin selection to the bucketed index, not \
             to the plain index covering the same fields"
        );

        let query = DriveDocumentCountQuery {
            document_type: *document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: DOCUMENT_TYPE.to_string(),
            index,
            where_clauses,
        };
        (query, bucket_start)
    }

    fn total_of(entries: &[SplitCountEntry]) -> u64 {
        entries.iter().map(|e| e.count.unwrap_or_default()).sum()
    }

    /// The headline case. Two `#ibiza` posts sit in the newest bucket and,
    /// because the window overlaps three deep, each is *stored* under three
    /// bucket keys. The verified count must be 2 — one per document — which
    /// is only true if both sides addressed exactly the one bucket the
    /// signed block time names.
    #[test]
    fn a_count_over_an_overlapping_bucket_verifies_and_counts_each_document_once() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);

        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        let transform = document_type
            .indexes()
            .get(BUCKETED_INDEX)
            .expect("the bucketed index survives contract registration")
            .time_range
            .as_ref()
            .expect("the bucketed index carries its transform");
        assert_eq!(
            transform.overlap_factor(),
            3,
            "the fixture's whole point is that windows overlap"
        );
        assert_eq!(
            transform.containing_buckets(POSTS[0].0).len(),
            3,
            "a matching document must really be stored under three bucket keys"
        );

        let request = trending_request(contract.id().to_vec(), "ibiza", select_count_star());
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let client_document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        let (query, bucket_start) =
            client_count_query(&contract, &client_document_type, "ibiza", mtd.time_ms);
        assert_eq!(
            bucket_start,
            expected_newest_bucket(&contract, mtd.time_ms),
            "the resolved bucket must be the transform's newest active start \
             for the signed time"
        );
        assert_eq!(bucket_start, NEWEST_BUCKET_START_MS);

        let entries = verify_point_lookup_count_proof(&query, &proof, &mtd, version, &provider)
            .expect("a correctly signed count over the resolved bucket must verify");

        assert_eq!(
            total_of(&entries),
            2,
            "the two #ibiza posts in the newest bucket count once each — a \
             document living in three overlapping buckets must not count three \
             times, and the #ibiza post one hour older belongs to the previous \
             buckets only"
        );
    }

    /// Move the signed time forward by one full step and the client resolves
    /// a *different* bucket. Verification must fail hard: either the proof
    /// cannot be replayed over the other bucket's path, or the tenderdash
    /// binding rejects the altered metadata. What must never happen is a
    /// second, differently-scoped count coming back verified.
    #[test]
    fn a_tampered_metadata_time_cannot_verify_a_different_bucket() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);

        let request = trending_request(contract.id().to_vec(), "ibiza", select_count_star());
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let step_ms = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists")
            .indexes()
            .get(BUCKETED_INDEX)
            .expect("the bucketed index survives contract registration")
            .time_range
            .as_ref()
            .expect("the bucketed index carries its transform")
            .step_ms();

        let mut tampered = mtd.clone();
        tampered.time_ms += step_ms;

        let client_document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        let (_honest_query, honest_bucket) =
            client_count_query(&contract, &client_document_type, "ibiza", mtd.time_ms);
        let (tampered_query, tampered_bucket) =
            client_count_query(&contract, &client_document_type, "ibiza", tampered.time_ms);
        assert_eq!(
            tampered_bucket,
            honest_bucket + step_ms,
            "one step of tampering must move the resolution one bucket — \
             otherwise this test proves nothing about where the bucket comes from"
        );

        let error =
            verify_point_lookup_count_proof(&tampered_query, &proof, &tampered, version, &provider)
                .expect_err("an altered signed time must not yield a verified count");
        assert!(
            matches!(
                error,
                ProofVerifierError::InvalidSignature { .. }
                    | ProofVerifierError::GroveDBError { .. }
                    | ProofVerifierError::DriveError { .. }
            ),
            "the rejection must be the proof or the signature binding, got: {error:?}"
        );
    }

    /// The non-aggregate route through the same fixture: the documents path
    /// resolves the selector, carries the provenance onto the drive query
    /// and verifies the returned rows. Same bucket scoping, different
    /// primitive — so a regression that only reached the documents index
    /// picker still turns a test red.
    #[test]
    fn a_documents_proof_over_the_newest_bucket_verifies_to_the_bucket_members() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, documents, state) = setup_trending(&platform, &base_state, version);

        let request = trending_request(contract.id().to_vec(), "ibiza", select_documents());
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        let mut where_clauses = vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("ibiza".to_string()),
        }];
        let (resolved_clause, resolution) = resolve_time_range_bucket_clause(
            CREATED_AT,
            TimeRangeSelector::Newest,
            None,
            document_type,
            mtd.time_ms,
        )
        .expect("the metadata time falls inside an active range");
        where_clauses.push(resolved_clause);
        let mut drive_query = DriveDocumentQuery::from_typed_clauses(
            where_clauses,
            Vec::new(),
            None,
            None,
            true,
            None,
            &contract,
            document_type,
            &platform.config.drive,
            version,
        )
        .expect("the resolved clause set builds a drive query");
        drive_query.resolved_time_ranges = vec![resolution];

        let response = GetDocumentsResponse {
            version: Some(ResponseVersion::V1(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(mtd),
            })),
        };
        let (verified, _mtd, _proof) =
            <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
                drive_query,
                response,
                Network::Testnet,
                version,
                &provider,
            )
            .expect("a correctly signed documents proof must verify");

        let mut verified_ids: Vec<_> = verified
            .expect("the newest bucket is not empty")
            .into_iter()
            .filter_map(|(id, document)| document.map(|_| id))
            .collect();
        verified_ids.sort();
        let mut expected_ids = vec![documents[0].id(), documents[1].id()];
        expected_ids.sort();
        assert_eq!(
            verified_ids, expected_ids,
            "only the two #ibiza posts inside the newest bucket are proven \
             members — the older #ibiza post and the #berlin post are not"
        );
    }

    // ----- the SDK `FromProof` entry points -------------------------------
    //
    // The tests above re-run the client sequence step by step so a failure
    // names the broken link. These run the same signed responses through the
    // aggregate `FromProof<DocumentQuery>` entry points the SDK actually
    // exposes — resolution from signed metadata time, provenance, shape
    // guard, index pick and proof verification all happen *inside* the call
    // — so a regression in how that layer wires the steps together (not just
    // in a step) turns a test red.

    use dash_platform_queries::documents::document_query::DocumentQuery as SdkDocumentQuery;
    use drive::query::SelectProjection;
    use drive_proof_verifier::{DocumentAverage, DocumentCount, DocumentSum};
    use std::sync::Arc;

    const SUMMABLE_INDEX: &str = "trendingLikes";

    /// `likes` per post, aligned with [`POSTS`]: the two newest-bucket
    /// `#ibiza` posts carry 10 and 30 (sum 40, average 20), the older
    /// `#ibiza` post carries 100 and the `#berlin` post 7 — both excluded,
    /// so an aggregate that leaked either is off by an unmistakable amount.
    const LIKES: [u64; 4] = [10, 30, 100, 7];

    /// The summable twin of [`register_trending_contract`]: the same
    /// six-hour/two-hour bucketed `(timeRange($createdAt), hashtag)` index,
    /// but countable *and* summable over a `likes` property — the CountSum
    /// value trees that SUM reads and AVG derives `(count, sum)` from. The
    /// plain `byHashtag` index mirrors both aggregate declarations so index
    /// selection again has a wrong-but-covering candidate to reject.
    fn register_engagement_contract(
        platform: &Platform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let factory = DataContractFactory::new(platform_version.protocol_version)
            .expect("expected a factory");
        let schemas = platform_value!({
            DOCUMENT_TYPE: {
                "type": "object",
                "properties": {
                    "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
                    "likes": { "type": "integer", "position": 1 },
                },
                "indices": [
                    {
                        "name": SUMMABLE_INDEX,
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "countable": true,
                        "summable": "likes",
                        "timeRange": { "on": "$createdAt", "range": 21_600u64, "step": 7_200u64 },
                    },
                    {
                        "name": "byHashtag",
                        "properties": [{ "hashtag": "asc" }, { "$createdAt": "asc" }],
                        "countable": true,
                        "summable": "likes",
                    },
                ],
                "required": ["$createdAt", "hashtag", "likes"],
                "additionalProperties": false,
            }
        });
        let contract = factory
            .create_with_value_config(Identifier::new([8u8; 32]), 0, schemas, None, None)
            .expect("the engagement contract is well-formed")
            .data_contract_owned();
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    /// [`insert_posts`] with a `likes` value from [`LIKES`] on each post.
    fn insert_engagement_posts(
        platform: &Platform<MockCoreRPCLike>,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) {
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        for (i, ((created_at_ms, hashtag), likes)) in POSTS.iter().zip(LIKES).enumerate() {
            let mut document: Document = document_type
                .random_document(Some(8_000 + i as u64), platform_version)
                .expect("random document");
            document.set_properties(BTreeMap::from([
                ("hashtag".to_string(), Value::Text(hashtag.to_string())),
                ("likes".to_string(), Value::U64(likes)),
            ]));
            document.set_created_at(Some(*created_at_ms));
            store_document(
                platform,
                contract,
                document_type,
                &document,
                platform_version,
            );
        }
    }

    /// Engagement contract registered, liked posts inserted, and a platform
    /// state whose committed block time is [`BLOCK_TIME_MS`].
    fn setup_engagement(
        platform: &TempPlatform<MockCoreRPCLike>,
        base_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> (DataContract, PlatformState) {
        let contract = register_engagement_contract(platform, platform_version);
        insert_engagement_posts(platform, &contract, platform_version);
        let state = state_with_committed_block_time(base_state, &platform.drive, platform_version);
        (contract, state)
    }

    /// The query a real SDK caller would build: the selector still pending in
    /// `time_range_clauses`, to be resolved by the entry point itself from
    /// the signed metadata time.
    fn sdk_query(
        contract: &DataContract,
        hashtag: &str,
        select: SelectProjection,
    ) -> SdkDocumentQuery {
        SdkDocumentQuery::new(Arc::new(contract.clone()), DOCUMENT_TYPE)
            .expect("the fixture has this document type")
            .with_select(select)
            .with_where(WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(hashtag.to_string()),
            })
            .with_time_range(CREATED_AT, TimeRangeSelector::Newest)
    }

    /// Wrap a handler proof and its metadata the way the wire does, so the
    /// entry points consume exactly what a node returns.
    fn signed_response(proof: Proof, mtd: &ResponseMetadata) -> GetDocumentsResponse {
        GetDocumentsResponse {
            version: Some(ResponseVersion::V1(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Proof(proof)),
                metadata: Some(mtd.clone()),
            })),
        }
    }

    fn select_field(function: v1_select::Function, field: &str) -> Vec<V1Select> {
        vec![V1Select {
            function: function as i32,
            field: field.to_string(),
        }]
    }

    /// Nudge the signed time one full step so the entry point's own
    /// resolution lands on a different bucket than the proof covers.
    fn one_step_later(
        contract: &DataContract,
        index: &str,
        mtd: &ResponseMetadata,
    ) -> ResponseMetadata {
        let step_ms = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists")
            .indexes()
            .get(index)
            .expect("the bucketed index survives contract registration")
            .time_range
            .as_ref()
            .expect("the bucketed index carries its transform")
            .step_ms();
        let mut tampered = mtd.clone();
        tampered.time_ms += step_ms;
        tampered
    }

    /// The rejection must come from proof reconstruction (GroveDB/Drive) —
    /// the caller re-signed the altered metadata, so `InvalidSignature`
    /// would mean the deeper property (resolve-later-bucket → mismatched
    /// proof path) was never exercised.
    fn assert_proof_path_rejection(error: ProofVerifierError) {
        assert!(
            matches!(
                error,
                ProofVerifierError::GroveDBError { .. } | ProofVerifierError::DriveError { .. }
            ),
            "the rejection must come from the proof path, got: {error:?}"
        );
    }

    #[test]
    fn a_count_proof_verifies_through_the_sdk_from_proof_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);

        let request = trending_request(contract.id().to_vec(), "ibiza", select_count_star());
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let query = sdk_query(&contract, "ibiza", SelectProjection::count_star());
        let (count, verified_mtd, _proof) =
            <DocumentCount as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                query,
                signed_response(proof, &mtd),
                Network::Testnet,
                version,
                &provider,
            )
            .expect("a correctly signed count must verify through the entry point");

        assert_eq!(
            count.expect("the newest bucket is not empty"),
            DocumentCount(2),
            "the two #ibiza posts in the newest bucket count once each through \
             the entry point, exactly as through the step-by-step sequence"
        );
        assert_eq!(
            verified_mtd.time_ms, BLOCK_TIME_MS,
            "the metadata handed back is the one the resolution consumed"
        );
    }

    #[test]
    fn a_tampered_metadata_time_is_rejected_at_the_count_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);

        let request = trending_request(contract.id().to_vec(), "ibiza", select_count_star());
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);
        let tampered = one_step_later(&contract, BUCKETED_INDEX, &mtd);

        // Re-sign the altered metadata so the signature is valid —
        // verification must then resolve the selector one step later,
        // reconstruct the NEXT bucket's path query, and reject the
        // original bucket's GroveDB proof. (Verification reconstructs the
        // proof path BEFORE checking the signature, so an unsigned
        // alteration would hit the same rejection without proving the
        // signature binds anything; the binding itself is pinned by the
        // trust-boundary tests above.)
        let resigned = resign_over(&platform, &proof, &tampered, version);
        let query = sdk_query(&contract, "ibiza", SelectProjection::count_star());
        let error = <DocumentCount as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
            query,
            signed_response(resigned, &tampered),
            Network::Testnet,
            version,
            &provider,
        )
        .expect_err("a validly re-signed later time must not verify the stale bucket's proof");
        assert_proof_path_rejection(error);
    }

    #[test]
    fn a_sum_proof_verifies_through_the_sdk_from_proof_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_engagement(&platform, &base_state, version);

        let request = GetDocumentsRequestV1 {
            selects: select_field(v1_select::Function::Sum, "likes"),
            ..trending_request(contract.id().to_vec(), "ibiza", Vec::new())
        };
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let query = sdk_query(&contract, "ibiza", SelectProjection::sum("likes"));
        let (sum, _mtd, _proof) =
            <DocumentSum as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                query,
                signed_response(proof, &mtd),
                Network::Testnet,
                version,
                &provider,
            )
            .expect("a correctly signed sum must verify through the entry point");

        assert_eq!(
            sum.expect("the newest bucket is not empty"),
            DocumentSum(40),
            "10 + 30 likes on the two newest-bucket #ibiza posts — the older \
             #ibiza post's 100 and the #berlin post's 7 must not leak in, and \
             overlap fan-out must not multiply the total"
        );
    }

    #[test]
    fn a_tampered_metadata_time_is_rejected_at_the_sum_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_engagement(&platform, &base_state, version);

        let request = GetDocumentsRequestV1 {
            selects: select_field(v1_select::Function::Sum, "likes"),
            ..trending_request(contract.id().to_vec(), "ibiza", Vec::new())
        };
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);
        let tampered = one_step_later(&contract, SUMMABLE_INDEX, &mtd);
        let resigned = resign_over(&platform, &proof, &tampered, version);

        let query = sdk_query(&contract, "ibiza", SelectProjection::sum("likes"));
        let error = <DocumentSum as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
            query,
            signed_response(resigned, &tampered),
            Network::Testnet,
            version,
            &provider,
        )
        .expect_err("a validly re-signed later time must not verify the stale bucket's proof");
        assert_proof_path_rejection(error);
    }

    #[test]
    fn an_average_proof_verifies_through_the_sdk_from_proof_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_engagement(&platform, &base_state, version);

        let request = GetDocumentsRequestV1 {
            selects: select_field(v1_select::Function::Avg, "likes"),
            ..trending_request(contract.id().to_vec(), "ibiza", Vec::new())
        };
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let query = sdk_query(&contract, "ibiza", SelectProjection::avg("likes"));
        let (average, _mtd, _proof) =
            <DocumentAverage as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                query,
                signed_response(proof, &mtd),
                Network::Testnet,
                version,
                &provider,
            )
            .expect("a correctly signed average must verify through the entry point");

        let average = average.expect("the newest bucket is not empty");
        assert_eq!(
            average,
            DocumentAverage { count: 2, sum: 40 },
            "the verified pair is the newest bucket's two #ibiza posts and \
             their 40 likes — nothing from outside the bucket, nothing doubled"
        );
        assert_eq!(average.as_f64(), Some(20.0));
    }

    #[test]
    fn a_tampered_metadata_time_is_rejected_at_the_average_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_engagement(&platform, &base_state, version);

        let request = GetDocumentsRequestV1 {
            selects: select_field(v1_select::Function::Avg, "likes"),
            ..trending_request(contract.id().to_vec(), "ibiza", Vec::new())
        };
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);
        let tampered = one_step_later(&contract, SUMMABLE_INDEX, &mtd);
        let resigned = resign_over(&platform, &proof, &tampered, version);

        let query = sdk_query(&contract, "ibiza", SelectProjection::avg("likes"));
        let error =
            <DocumentAverage as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                query,
                signed_response(resigned, &tampered),
                Network::Testnet,
                version,
                &provider,
            )
            .expect_err("a validly re-signed later time must not verify the stale bucket's proof");
        assert_proof_path_rejection(error);
    }
    // ----- multiple grids over one timestamp ------------------------------

    use drive::query::TimeRangeGridSpec;

    const DAILY_INDEX: &str = "daily";
    const DAY_SECONDS: u64 = 24 * 3_600;

    /// The trending contract plus a second, daily (24h/24h) grid over the
    /// same `$createdAt`. Grid-qualified level keys give each grid its own
    /// subtree; the tests below pin that the wire can address each grid and
    /// that the two prove independently.
    fn register_two_grid_contract(
        platform: &Platform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let factory = DataContractFactory::new(platform_version.protocol_version)
            .expect("expected a factory");
        let schemas = platform_value!({
            DOCUMENT_TYPE: {
                "type": "object",
                "properties": {
                    "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
                },
                "indices": [
                    {
                        "name": BUCKETED_INDEX,
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "countable": true,
                        "timeRange": { "on": "$createdAt", "range": 21_600u64, "step": 7_200u64 },
                    },
                    {
                        "name": DAILY_INDEX,
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "countable": true,
                        "timeRange": { "on": "$createdAt", "range": DAY_SECONDS, "step": DAY_SECONDS },
                    },
                ],
                "required": ["$createdAt", "hashtag"],
                "additionalProperties": false,
            }
        });
        let contract = factory
            .create_with_value_config(Identifier::new([9u8; 32]), 0, schemas, None, None)
            .expect("a contract may bucket one timestamp with several grids")
            .data_contract_owned();
        store_data_contract(platform, &contract, platform_version);
        contract
    }

    fn setup_two_grids(
        platform: &TempPlatform<MockCoreRPCLike>,
        base_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> (DataContract, PlatformState) {
        let contract = register_two_grid_contract(platform, platform_version);
        insert_posts(platform, &contract, platform_version);
        let state = state_with_committed_block_time(base_state, &platform.drive, platform_version);
        (contract, state)
    }

    /// The wire's structured `IN_TIME_RANGE` operand:
    /// `[selector, range, step]` (seconds, as the contract declares them).
    fn structured_where_clauses(hashtag: &str, grid: TimeRangeGridSpec) -> Vec<ProtoWhereClause> {
        let uint = |n: u64| ProtoDocumentFieldValue {
            variant: Some(document_field_value::Variant::Uint64Value(n)),
        };
        let mut values = vec![
            ProtoDocumentFieldValue {
                variant: Some(document_field_value::Variant::Text(
                    TimeRangeSelector::Newest.as_str().to_string(),
                )),
            },
            uint(grid.range_seconds),
            uint(grid.step_seconds),
        ];
        if grid.phase_seconds != 0 {
            values.push(uint(grid.phase_seconds));
        }
        vec![
            wc(
                "hashtag",
                ProtoWhereOperator::Equal,
                Value::Text(hashtag.to_string()),
            ),
            ProtoWhereClause {
                field: CREATED_AT.to_string(),
                operator: ProtoWhereOperator::InTimeRange as i32,
                value: Some(ProtoDocumentFieldValue {
                    variant: Some(document_field_value::Variant::List(
                        document_field_value::ValueList { values },
                    )),
                }),
            },
        ]
    }

    const TRENDING_GRID: TimeRangeGridSpec = TimeRangeGridSpec {
        range_seconds: 21_600,
        step_seconds: 7_200,
        phase_seconds: 0,
    };
    const DAILY_GRID: TimeRangeGridSpec = TimeRangeGridSpec {
        range_seconds: DAY_SECONDS,
        step_seconds: DAY_SECONDS,
        phase_seconds: 0,
    };

    /// With two grids on `$createdAt`, the bare text selector no longer
    /// names a grid, so the handler refuses it as ambiguous rather than
    /// picking one — a silent pick would prove an answer to a question the
    /// client didn't ask.
    #[test]
    fn a_bare_selector_on_a_multi_grid_field_is_refused_as_ambiguous() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_two_grids(&platform, &base_state, version);

        let request = trending_request(contract.id().to_vec(), "ibiza", select_count_star());
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("transport-level success");
        assert!(
            !result.errors.is_empty(),
            "a bare selector over two grids must be refused"
        );
        assert!(
            format!("{:?}", result.errors).contains("grids"),
            "the refusal must say the field is multi-grid: {:?}",
            result.errors
        );
    }

    /// A four-element operand spelling a zero phase is refused: like the
    /// contract grammar and the storage key, zero is spelled by omission so
    /// every grid has exactly one wire spelling.
    #[test]
    fn an_explicit_zero_phase_operand_is_refused_as_non_canonical() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_two_grids(&platform, &base_state, version);

        let mut clauses = structured_where_clauses("ibiza", TRENDING_GRID);
        // append an explicit zero phase to the list operand
        if let Some(ProtoDocumentFieldValue {
            variant: Some(document_field_value::Variant::List(list)),
        }) = clauses[1].value.as_mut()
        {
            list.values.push(ProtoDocumentFieldValue {
                variant: Some(document_field_value::Variant::Uint64Value(0)),
            });
        } else {
            panic!("the helper builds a list operand");
        }
        let request = GetDocumentsRequestV1 {
            where_clauses: clauses,
            ..trending_request(contract.id().to_vec(), "ibiza", select_count_star())
        };
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("transport-level success");
        assert!(
            format!("{:?}", result.errors).contains("omission"),
            "expected the one-spelling-per-grid refusal, got {:?}",
            result.errors
        );
    }

    /// Each grid proves and verifies independently through the SDK
    /// `FromProof` entry point, with the structured operand naming the grid
    /// on the wire and `with_time_range_grid` naming it in the client query.
    /// The counts differ by design: the newest trending window holds two
    /// `#ibiza` posts, while the newest daily window also contains the
    /// one-hour-older post — three. A collapsed keyspace could not produce
    /// both answers.
    #[test]
    fn each_grid_proves_and_verifies_independently_through_the_entry_point() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, state) = setup_two_grids(&platform, &base_state, version);

        for (grid, expected_count) in [(TRENDING_GRID, 2u64), (DAILY_GRID, 3u64)] {
            let request = GetDocumentsRequestV1 {
                where_clauses: structured_where_clauses("ibiza", grid),
                ..trending_request(contract.id().to_vec(), "ibiza", select_count_star())
            };
            let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

            let query = SdkDocumentQuery::new(Arc::new(contract.clone()), DOCUMENT_TYPE)
                .expect("the fixture has this document type")
                .with_select(SelectProjection::count_star())
                .with_where(WhereClause {
                    field: "hashtag".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("ibiza".to_string()),
                })
                .with_time_range_grid(CREATED_AT, TimeRangeSelector::Newest, grid);
            let (count, _mtd, _proof) =
                <DocumentCount as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                    query,
                    signed_response(proof, &mtd),
                    Network::Testnet,
                    version,
                    &provider,
                )
                .expect("a correctly signed per-grid count must verify");
            assert_eq!(
                count.expect("the bucket is not empty"),
                DocumentCount(expected_count),
                "grid {:?} must count its own bucket's members",
                grid
            );
        }
    }
    // ----- routes that must refuse time-range selections ------------------

    use drive_proof_verifier::{DocumentHavingEntries, DocumentRankedEntries};

    /// The HAVING route accepts equality prefixes that pin plain ranked
    /// indexes, and its picker excludes transformed ones — so a resolved
    /// bucket-start equality reaching it would be served from raw
    /// timestamps at the bucket boundary instead of the selected window.
    /// The server must refuse the combination outright (drive owns the
    /// rejection, through `DocumentHavingRequest::resolved_time_ranges`).
    #[test]
    fn the_having_route_refuses_a_time_range_selection() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);

        let request = GetDocumentsRequestV1 {
            group_by: vec!["hashtag".to_string()],
            having: vec![hc(
                having_aggregate::Function::Count,
                "",
                having_clause::Operator::GreaterThan,
                Value::U64(0),
            )],
            ..trending_request(contract.id().to_vec(), "ibiza", select_count_star())
        };
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("transport-level success");
        assert!(
            format!("{:?}", result.errors).contains("time-range"),
            "the HAVING route must refuse a time-range selection, got {:?}",
            result.errors
        );
    }

    /// The ranked and HAVING *verifiers* must refuse a time-range request
    /// before authenticating anything: both surfaces pin plain ranked
    /// indexes with equality prefixes, so a malicious node could otherwise
    /// prove raw-timestamp matches at the bucket boundary against a request
    /// shape every honest server rejects. The rejection fires ahead of
    /// proof verification, so any signed response works as the fixture.
    #[test]
    fn ranked_and_having_verifiers_refuse_time_range_requests() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);
        let request = trending_request(contract.id().to_vec(), "ibiza", select_count_star());
        let (proof, mtd, provider) = prove_and_sign(&platform, &state, request, version);

        let time_range_query = || {
            SdkDocumentQuery::new(Arc::new(contract.clone()), DOCUMENT_TYPE)
                .expect("the fixture has this document type")
                .with_select(SelectProjection::count_star())
                .with_time_range(CREATED_AT, TimeRangeSelector::Newest)
        };

        let error =
            <DocumentRankedEntries as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                time_range_query(),
                signed_response(proof.clone(), &mtd),
                Network::Testnet,
                version,
                &provider,
            )
            .expect_err("the ranked verifier must refuse a time-range request");
        assert!(
            error.to_string().contains("time-range"),
            "expected the ranked time-range refusal, got {error}"
        );

        let error =
            <DocumentHavingEntries as FromProof<SdkDocumentQuery>>::maybe_from_proof_with_metadata(
                time_range_query().with_having(vec![drive::query::HavingClause {
                    aggregate: drive::query::HavingAggregate {
                        function: drive::query::HavingAggregateFunction::Count,
                        field: String::new(),
                    },
                    operator: drive::query::HavingOperator::GreaterThan,
                    right: drive::query::HavingRightOperand::Value(Value::U64(0)),
                }]),
                signed_response(proof, &mtd),
                Network::Testnet,
                version,
                &provider,
            )
            .expect_err("the HAVING verifier must refuse a time-range request");
        assert!(
            error.to_string().contains("time-range"),
            "expected the HAVING time-range refusal, got {error}"
        );
    }

    /// A drive query carrying resolution provenance has no faithful
    /// `DocumentQuery` form — serializing it would demote the resolved
    /// bucket equality to a raw-timestamp predicate — so the conversion
    /// must refuse rather than silently rewrite the question.
    #[test]
    fn a_resolved_drive_query_cannot_convert_to_a_document_query() {
        let (platform, base_state, version) = setup_platform(None, Network::Testnet, None);
        let (contract, _documents, state) = setup_trending(&platform, &base_state, version);
        let document_type = contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists");
        let block_time_ms = state
            .last_committed_block_time_ms()
            .expect("the fixture committed a block");
        let (clause, resolution) = resolve_time_range_bucket_clause(
            CREATED_AT,
            TimeRangeSelector::Newest,
            None,
            document_type,
            block_time_ms,
        )
        .expect("the committed block time is inside an active range");

        let mut drive_query = DriveDocumentQuery::from_typed_clauses(
            vec![clause],
            Vec::new(),
            None,
            None,
            true,
            None,
            &contract,
            document_type,
            &platform.config.drive,
            version,
        )
        .expect("the resolved clause builds a drive query");
        drive_query.resolved_time_ranges = vec![resolution];

        let error = SdkDocumentQuery::try_from(&drive_query)
            .expect_err("resolution provenance must not survive the conversion");
        assert!(
            error.to_string().contains("provenance"),
            "expected the provenance rejection, got {error}"
        );
    }
}
