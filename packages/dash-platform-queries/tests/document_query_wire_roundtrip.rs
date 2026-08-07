//! Round-trip tests for the wire codec of [`DocumentQuery`]:
//! `DocumentQuery` → [`GetDocumentsRequest`] →
//! [`DocumentQuery::try_from_request`] must reproduce the original
//! query exactly, on both the V0 (CBOR clause) and V1 (typed proto
//! clause) wire encodings — this is what lets an embedder verify a
//! proved response against nothing but the request bytes it sent.

use std::sync::Arc;

use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dapi_grpc::platform::v0::get_documents_request::{GetDocumentsRequestV0, Version};
use dapi_grpc::platform::v0::GetDocumentsRequest;
use dash_platform_queries::documents::document_query::DocumentQuery;
use dash_platform_queries::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::query::{OrderClause, SelectProjection, WhereClause, WhereOperator};

fn test_contract() -> Arc<DataContract> {
    let platform_version = PlatformVersion::latest();
    Arc::new(
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned(),
    )
}

/// A protocol version whose `drive_abci.query.document_query`
/// feature-version is `0` — encodes onto the V0 (CBOR-clause) wire.
fn v0_platform_version() -> &'static PlatformVersion {
    let version = PlatformVersion::get(1).expect("protocol version 1 exists");
    assert_eq!(
        version
            .drive_abci
            .query
            .document_query
            .default_current_version,
        0,
        "protocol version 1 should encode the V0 documents wire"
    );
    version
}

/// The latest protocol version — encodes onto the V1 (typed proto
/// clause) wire.
fn v1_platform_version() -> &'static PlatformVersion {
    let version = PlatformVersion::latest();
    assert_eq!(
        version
            .drive_abci
            .query
            .document_query
            .default_current_version,
        1,
        "latest protocol version should encode the V1 documents wire"
    );
    version
}

fn roundtrip(query: &DocumentQuery, platform_version: &PlatformVersion) -> DocumentQuery {
    let contract = Arc::clone(&query.data_contract);
    let request = query
        .clone()
        .try_into_request_for_version(platform_version)
        .expect("query should encode onto the wire");
    DocumentQuery::try_from_request(request, contract)
        .expect("wire request should decode back into a query")
}

#[test]
fn v1_roundtrip_documents_query_full_surface() {
    let contract = test_contract();
    let mut query = DocumentQuery::new(Arc::clone(&contract), "niceDocument")
        .expect("document type exists")
        .with_where(WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Alice".to_string()),
        })
        .with_where(WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(21), Value::U64(42)]),
        })
        .with_where(WhereClause {
            field: "balance".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::I64(-5),
        })
        .with_order_by(OrderClause {
            field: "firstName".to_string(),
            ascending: true,
        })
        .with_order_by(OrderClause {
            field: "age".to_string(),
            ascending: false,
        })
        .with_limit(42)
        .with_offset(7);
    query.start = Some(Start::StartAt(vec![1u8; 32]));

    assert_eq!(roundtrip(&query, v1_platform_version()), query);
}

#[test]
fn v1_roundtrip_start_after() {
    let contract = test_contract();
    let mut query = DocumentQuery::new(contract, "niceDocument").expect("document type exists");
    query.start = Some(Start::StartAfter(vec![2u8; 32]));

    assert_eq!(roundtrip(&query, v1_platform_version()), query);
}

#[test]
fn v1_roundtrip_grouped_count() {
    let contract = test_contract();
    let query = DocumentQuery::new(contract, "niceDocument")
        .expect("document type exists")
        .with_select(SelectProjection::count_star())
        .with_group_by("age")
        .with_where(WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::GreaterThanOrEquals,
            value: Value::U64(18),
        })
        .with_limit(5);

    assert_eq!(roundtrip(&query, v1_platform_version()), query);
}

#[test]
fn v0_roundtrip_documents_query() {
    let contract = test_contract();
    let mut query = DocumentQuery::new(Arc::clone(&contract), "niceDocument")
        .expect("document type exists")
        .with_where(WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Alice".to_string()),
        })
        .with_where(WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(21), Value::U64(42)]),
        })
        .with_where(WhereClause {
            field: "balance".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::I64(-5),
        })
        .with_order_by(OrderClause {
            field: "firstName".to_string(),
            ascending: true,
        })
        .with_order_by(OrderClause {
            field: "age".to_string(),
            ascending: false,
        })
        .with_limit(10);
    query.start = Some(Start::StartAfter(vec![3u8; 32]));

    let request = query
        .clone()
        .try_into_request_for_version(v0_platform_version())
        .expect("query should encode onto the V0 wire");
    assert!(
        matches!(request.version, Some(Version::V0(_))),
        "protocol version 1 must produce the V0 wire shape"
    );
    let decoded = DocumentQuery::try_from_request(request, contract)
        .expect("V0 wire request should decode back into a query");
    assert_eq!(decoded, query);
}

#[test]
fn v0_rejects_malformed_where_cbor() {
    let contract = test_contract();
    let request = GetDocumentsRequest {
        version: Some(Version::V0(GetDocumentsRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "niceDocument".to_string(),
            r#where: vec![0x9F], // truncated CBOR array
            order_by: vec![],
            limit: 0,
            prove: true,
            start: None,
        })),
    };

    let error = DocumentQuery::try_from_request(request, contract)
        .expect_err("truncated where CBOR must be rejected");
    assert!(
        error.to_string().contains("unable to decode 'where' query"),
        "unexpected error: {error}"
    );
}

#[test]
fn v1_rejects_unknown_where_operator() {
    let contract = test_contract();
    let query = DocumentQuery::new(Arc::clone(&contract), "niceDocument")
        .expect("document type exists")
        .with_where(WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("Alice".to_string()),
        });
    let mut request = query
        .try_into_request_for_version(v1_platform_version())
        .expect("query should encode onto the wire");
    let Some(Version::V1(request_v1)) = &mut request.version else {
        panic!("expected the V1 wire shape");
    };
    request_v1.where_clauses[0].operator = 99;

    let error = DocumentQuery::try_from_request(request, contract)
        .expect_err("unknown operator discriminant must be rejected");
    assert!(
        error
            .to_string()
            .contains("unknown WhereOperator discriminant: 99"),
        "unexpected error: {error}"
    );
}

#[test]
fn v1_rejects_explicit_zero_limit() {
    let contract = test_contract();
    let query =
        DocumentQuery::new(Arc::clone(&contract), "niceDocument").expect("document type exists");
    let mut request = query
        .try_into_request_for_version(v1_platform_version())
        .expect("query should encode onto the wire");
    let Some(Version::V1(request_v1)) = &mut request.version else {
        panic!("expected the V1 wire shape");
    };
    request_v1.limit = Some(0);

    let error = DocumentQuery::try_from_request(request, contract)
        .expect_err("explicit zero limit must be rejected, mirroring the server");
    assert!(
        error.to_string().contains("limit = 0"),
        "unexpected error: {error}"
    );
}

#[test]
fn v1_rejects_multi_projection_select() {
    let contract = test_contract();
    let query =
        DocumentQuery::new(Arc::clone(&contract), "niceDocument").expect("document type exists");
    let mut request = query
        .try_into_request_for_version(v1_platform_version())
        .expect("query should encode onto the wire");
    let Some(Version::V1(request_v1)) = &mut request.version else {
        panic!("expected the V1 wire shape");
    };
    let extra_select = request_v1.selects[0].clone();
    request_v1.selects.push(extra_select);

    let error = DocumentQuery::try_from_request(request, contract)
        .expect_err("multi-projection SELECT must be rejected");
    assert!(
        error.to_string().contains("multi-projection SELECT"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_contract_mismatch() {
    let contract = test_contract();
    let query =
        DocumentQuery::new(Arc::clone(&contract), "niceDocument").expect("document type exists");
    let mut request = query
        .try_into_request_for_version(v1_platform_version())
        .expect("query should encode onto the wire");
    let Some(Version::V1(request_v1)) = &mut request.version else {
        panic!("expected the V1 wire shape");
    };
    request_v1.data_contract_id = vec![9u8; 32];

    let error = DocumentQuery::try_from_request(request, contract)
        .expect_err("mismatched contract id must be rejected");
    assert!(
        matches!(error, Error::Protocol(_)),
        "unexpected error: {error}"
    );
    assert!(
        error.to_string().contains("targets data contract"),
        "unexpected error: {error}"
    );
}
