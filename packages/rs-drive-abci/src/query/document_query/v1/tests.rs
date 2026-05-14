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
    Select as V1Select, Start as V1Start,
};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::platform_value::platform_value;

fn empty_v1_request() -> GetDocumentsRequestV1 {
    GetDocumentsRequestV1 {
        data_contract_id: vec![0u8; 32],
        document_type: "widget".to_string(),
        r#where: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: None,
        prove: false,
        select: V1Select::Documents as i32,
        group_by: Vec::new(),
        having: Vec::new(),
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
    let request = GetDocumentsRequestV1 {
        having: vec![0x01, 0x02],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(validate_and_route_for_tests(&request, &[]), "HAVING clause");
}

/// Unknown `Select` enum discriminants (e.g. `42`) are malformed
/// wire input, not future capability. The handler must classify
/// them as [`QueryError::InvalidArgument`] — `not_yet_implemented`
/// carries the contract "valid request shape, caller can keep it
/// unchanged when capability lands" which is wrong for garbage
/// enum discriminants (no future protocol value would make `42`
/// meaningful for `Select`).
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
        // outside the `Select` enum's defined set.
        select: 42,
        ..empty_v1_request()
    };
    match validate_and_route_for_tests(&request, &[]) {
        Err(QueryError::InvalidArgument(msg)) => {
            assert!(
                msg.contains("42") && msg.contains("Select"),
                "expected invalid-discriminant message naming the value and the \
                 enum, got: {}",
                msg
            );
        }
        Err(QueryError::Query(QuerySyntaxError::Unsupported(msg))) => panic!(
            "expected InvalidArgument for unknown Select discriminant; got \
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
                select: V1Select::Documents as i32,
                limit: Some(0),
                ..empty_v1_request()
            },
            Vec::new(),
        ),
        (
            "SELECT COUNT, group_by=[] (Aggregate) with In clause",
            GetDocumentsRequestV1 {
                select: V1Select::Count as i32,
                limit: Some(0),
                ..empty_v1_request()
            },
            in_clauses(),
        ),
        (
            "SELECT COUNT, group_by=[in_field] (GroupByIn)",
            GetDocumentsRequestV1 {
                select: V1Select::Count as i32,
                group_by: vec!["brand".to_string()],
                limit: Some(0),
                ..empty_v1_request()
            },
            in_clauses(),
        ),
        (
            "SELECT COUNT, group_by=[range_field] (GroupByRange)",
            GetDocumentsRequestV1 {
                select: V1Select::Count as i32,
                group_by: vec!["color".to_string()],
                limit: Some(0),
                ..empty_v1_request()
            },
            range_clauses(),
        ),
        (
            "SELECT COUNT, group_by=[in_field, range_field] (GroupByCompound)",
            GetDocumentsRequestV1 {
                select: V1Select::Count as i32,
                group_by: vec!["brand".to_string(), "color".to_string()],
                limit: Some(0),
                ..empty_v1_request()
            },
            in_and_range_clauses(),
        ),
    ];

    for (label, request, where_clauses) in cases {
        match validate_and_route_for_tests(&request, &where_clauses) {
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

#[test]
fn reject_group_by_with_documents() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Documents as i32,
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[]),
        "GROUP BY with SELECT DOCUMENTS",
    );
}

#[test]
fn reject_group_by_field_not_in_where_clauses() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[]),
        "GROUP BY on field 'color' which is not constrained",
    );
}

#[test]
fn reject_group_by_more_than_two_fields() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
        group_by: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        ..empty_v1_request()
    };
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &[]),
        "GROUP BY with more than two fields",
    );
}

#[test]
fn reject_two_field_group_by_outside_compound_shape() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
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
        validate_and_route_for_tests(&request, &where_clauses),
        "two-field GROUP BY outside the `(In, range)` compound shape",
    );
}

#[test]
fn accept_count_with_empty_group_by_routes_to_aggregate() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
        ..empty_v1_request()
    };
    assert_eq!(
        validate_and_route_for_tests(&request, &[]).unwrap(),
        "count_aggregate"
    );
}

#[test]
fn reject_count_aggregate_with_limit() {
    // Aggregate count is a single row; a `limit` is structurally
    // meaningless and previously caused Drive's per-In fan-out
    // to honor it and return a partial sum disguised as a total.
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
        limit: Some(1),
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: platform_value!([30u32, 40u32]),
    }];
    match validate_and_route_for_tests(&request, &where_clauses) {
        Err(QueryError::Query(QuerySyntaxError::InvalidLimit(msg))) => {
            assert!(
                msg.contains("aggregate count is a single row"),
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
        select: V1Select::Count as i32,
        group_by: vec!["age".to_string()],
        limit: Some(1),
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "age".to_string(),
        operator: WhereOperator::In,
        value: platform_value!([30u32, 40u32, 50u32]),
    }];
    match validate_and_route_for_tests(&request, &where_clauses) {
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
fn reject_single_field_group_by_on_in_field_when_range_also_constrained() {
    // `group_by=[in_field]` looks well-formed in isolation, but
    // the simultaneous range clause forces Drive's compound walk
    // to emit `(in_key, key)` rows that don't match the caller's
    // single-field grouping. Caller must spell out the compound
    // shape explicitly with `[in_field, range_field]`.
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
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
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &where_clauses),
        "single-field GROUP BY when both `In` and range clauses are present",
    );
}

#[test]
fn reject_single_field_group_by_on_range_field_when_in_also_constrained() {
    // Mirror of the above for the range-field branch: same
    // compound-shape mismatch, different `group_by` entry.
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
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
    assert_not_yet_implemented(
        validate_and_route_for_tests(&request, &where_clauses),
        "single-field GROUP BY when both `In` and range clauses are present",
    );
}

#[test]
fn accept_count_group_by_in_field_routes_to_in_entries() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
        group_by: vec!["brand".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "brand".to_string(),
        operator: WhereOperator::In,
        value: platform_value!(["acme", "contoso"]),
    }];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses).unwrap(),
        "count_entries_via_in_field"
    );
}

#[test]
fn accept_count_group_by_range_field_routes_to_range_entries() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
        group_by: vec!["color".to_string()],
        ..empty_v1_request()
    };
    let where_clauses = vec![WhereClause {
        field: "color".to_string(),
        operator: WhereOperator::GreaterThan,
        value: platform_value!("blue"),
    }];
    assert_eq!(
        validate_and_route_for_tests(&request, &where_clauses).unwrap(),
        "count_entries_via_range_field"
    );
}

#[test]
fn accept_count_group_by_compound_routes_to_compound_entries() {
    let request = GetDocumentsRequestV1 {
        select: V1Select::Count as i32,
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
        validate_and_route_for_tests(&request, &where_clauses).unwrap(),
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
        r#where: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: None,
        prove: false,
        select: V1Select::Documents as i32,
        group_by: Vec::new(),
        having: Vec::new(),
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
        r#where: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: None,
        prove: false,
        select: V1Select::Count as i32,
        group_by: Vec::new(),
        having: vec![0xFF, 0xFE],
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
        r#where: Vec::new(),
        order_by: Vec::new(),
        limit: None,
        start: Some(V1Start::StartAfter(vec![1u8; 32])),
        prove: false,
        select: V1Select::Count as i32,
        group_by: Vec::new(),
        having: Vec::new(),
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
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select as V1Select;
    use dpp::dashcore::Network;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::DocumentV0Setters;
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

    fn serialize_where_clauses_to_cbor(where_clauses: Vec<Value>) -> Vec<u8> {
        use ciborium::value::Value as CborValue;
        let cbor: CborValue = TryInto::<CborValue>::try_into(Value::Array(where_clauses))
            .expect("expected to convert where clauses to cbor value");
        let mut out = Vec::new();
        ciborium::ser::into_writer(&cbor, &mut out).expect("expected to serialize where clauses");
        out
    }

    fn store_person_document(
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
        where_bytes: Vec<u8>,
        order_by_bytes: Vec<u8>,
        group_by: Vec<String>,
        limit: Option<u32>,
        prove: bool,
    ) -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id,
            document_type: document_type.to_string(),
            r#where: where_bytes,
            order_by: order_by_bytes,
            limit,
            start: None,
            prove,
            select: V1Select::Count as i32,
            group_by,
            having: Vec::new(),
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

        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
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

        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text(">".to_string()),
            Value::U64(20),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
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

        let where_clauses = vec![Value::Array(vec![
            Value::Text("firstName".to_string()),
            Value::Text("==".to_string()),
            Value::Text("Alice".to_string()),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
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

        let where_clauses = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("in".to_string()),
            Value::Array(vec![Value::U64(30), Value::U64(40)]),
        ])];
        let order_by = vec![Value::Array(vec![
            Value::Text("age".to_string()),
            Value::Text("asc".to_string()),
        ])];

        let request = count_v1_request(
            data_contract.id().to_vec(),
            "person",
            serialize_where_clauses_to_cbor(where_clauses),
            serialize_where_clauses_to_cbor(order_by),
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
            let where_clauses = vec![Value::Array(vec![
                Value::Text("color".to_string()),
                Value::Text(">".to_string()),
                Value::Text("blue".to_string()),
            ])];
            let order_by_bytes = match ascending {
                Some(asc) => serialize_where_clauses_to_cbor(vec![Value::Array(vec![
                    Value::Text("color".to_string()),
                    Value::Text(if asc { "asc" } else { "desc" }.to_string()),
                ])]),
                None => Vec::new(),
            };
            count_v1_request(
                contract.id().to_vec(),
                "widget",
                serialize_where_clauses_to_cbor(where_clauses),
                order_by_bytes,
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

        let where_clauses = vec![Value::Array(vec![
            Value::Text("color".to_string()),
            Value::Text(">".to_string()),
            Value::Text("blue".to_string()),
        ])];
        let request = count_v1_request(
            contract.id().to_vec(),
            "widget",
            serialize_where_clauses_to_cbor(where_clauses),
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
