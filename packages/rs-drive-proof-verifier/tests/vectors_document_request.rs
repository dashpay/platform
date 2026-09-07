//! Request-driven document verification (`FromProof<GetDocumentsRequest>`),
//! replayed against the proof-vector corpus.
//!
//! The documents fixtures store placeholder payloads at the document
//! positions, so (as in `vectors_documents.rs`) the positive half pins that
//! the wire request decodes into exactly the `DriveDocumentQuery` the fixture
//! proof was generated for: the grovedb layer verifies to the pinned root
//! hash, and `FromProof` then fails cleanly at document decode. The negative
//! half pins the request-shape gates: shapes an honest server would never
//! have answered with a plain proved document set are refused before any
//! proof machinery runs.

#![cfg(feature = "mocks")]

mod common;

use common::{identifier, load_case, Case, Expected, RequestSpec, NETWORK};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    select, Select as ProtoSelect,
};
use dapi_grpc::platform::v0::get_documents_request::{
    document_field_value, get_documents_request_v1::Start as V1Start, DocumentFieldValue,
    GetDocumentsRequestV0, GetDocumentsRequestV1, OrderClause as ProtoOrderClause, Version,
    WhereClause as ProtoWhereClause, WhereOperator as ProtoWhereOperator,
};
use dapi_grpc::platform::v0::{self as platform, get_documents_response, GetDocumentsRequest};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Value;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use drive_proof_verifier::{DocumentWireQuery, Error, FromProof, RequestedDocuments};

fn text(value: &str) -> DocumentFieldValue {
    DocumentFieldValue {
        variant: Some(document_field_value::Variant::Text(value.to_string())),
    }
}

fn identifier_value(hex: &str) -> DocumentFieldValue {
    DocumentFieldValue {
        variant: Some(document_field_value::Variant::BytesValue(
            identifier(hex).to_vec(),
        )),
    }
}

fn equal(field: &str, value: DocumentFieldValue) -> ProtoWhereClause {
    ProtoWhereClause {
        field: field.to_string(),
        operator: ProtoWhereOperator::Equal as i32,
        value: Some(value),
        time_range: None,
    }
}

fn asc(field: &str) -> ProtoOrderClause {
    ProtoOrderClause {
        target: Some(
            dapi_grpc::platform::v0::get_documents_request::order_clause::Target::Field(
                field.to_string(),
            ),
        ),
        ascending: true,
    }
}

/// The v1 wire request the Dash Core transport sends for each fixture case
/// (same shapes `vectors_documents.rs` pins as hand-built drive queries).
fn v1_request(case: &Case, contract_id: Vec<u8>) -> GetDocumentsRequest {
    let (document_type, where_clauses, order_by, limit) = match &case.manifest.request {
        RequestSpec::DocumentsDpnsExact {
            normalized_label,
            limit,
        } => (
            "domain",
            vec![
                equal("normalizedParentDomainName", text("dash")),
                equal("normalizedLabel", text(normalized_label)),
            ],
            vec![],
            *limit,
        ),
        RequestSpec::DocumentsDpnsPrefix {
            normalized_prefix,
            limit,
        } => (
            "domain",
            vec![
                equal("normalizedParentDomainName", text("dash")),
                ProtoWhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: ProtoWhereOperator::StartsWith as i32,
                    value: Some(text(normalized_prefix)),
                    time_range: None,
                },
            ],
            vec![asc("normalizedLabel")],
            *limit,
        ),
        RequestSpec::DocumentsDashpayProfile { owner_id } => (
            "profile",
            vec![equal("$ownerId", identifier_value(owner_id))],
            vec![],
            1,
        ),
        RequestSpec::DocumentsDashpayContacts {
            identity_id,
            to_identity,
            limit,
        } => (
            "contactRequest",
            vec![equal(
                if *to_identity { "toUserId" } else { "$ownerId" },
                identifier_value(identity_id),
            )],
            vec![asc("$createdAt")],
            *limit,
        ),
        _ => panic!("{}: not a documents request", case.name),
    };
    GetDocumentsRequest {
        version: Some(Version::V1(GetDocumentsRequestV1 {
            data_contract_id: contract_id,
            document_type: document_type.to_string(),
            where_clauses,
            order_by,
            limit: Some(u32::from(limit)),
            start: None,
            prove: true,
            selects: vec![],
            group_by: vec![],
            having: vec![],
            offset: None,
            chained: None,
            ..Default::default()
        })),
    }
}

fn contract_for(case: &Case) -> SystemDataContract {
    match &case.manifest.request {
        RequestSpec::DocumentsDpnsExact { .. } | RequestSpec::DocumentsDpnsPrefix { .. } => {
            SystemDataContract::DPNS
        }
        RequestSpec::DocumentsDashpayProfile { .. }
        | RequestSpec::DocumentsDashpayContacts { .. } => SystemDataContract::Dashpay,
        _ => panic!("{}: not a documents request", case.name),
    }
}

fn response(case: &Case) -> platform::GetDocumentsResponse {
    platform::GetDocumentsResponse {
        version: Some(get_documents_response::Version::V1(
            get_documents_response::GetDocumentsResponseV1 {
                metadata: Some(case.metadata()),
                result: Some(
                    get_documents_response::get_documents_response_v1::Result::Proof(
                        case.grpc_proof(),
                    ),
                ),
            },
        )),
    }
}

fn run_case(name: &str) {
    let case = load_case(name);
    let Expected::DocumentsPlaceholder { .. } = &case.manifest.expected else {
        panic!("{name}: manifest expectation mismatch");
    };
    let platform_version = case.platform_version();
    let contract = load_system_data_contract(contract_for(&case), platform_version)
        .expect("load system data contract");
    let request = v1_request(&case, contract.id().to_vec());

    // The decoded request must lower to the query the fixture proof was made
    // for: the grovedb layer verifies to the pinned root hash.
    let wire_query = <DocumentWireQuery as drive_proof_verifier::from_request::TryFromRequest<
        GetDocumentsRequest,
    >>::try_from_request(request.clone())
    .expect("decode wire request");
    let drive_query = wire_query
        .to_drive_query(&contract, None, platform_version)
        .expect("lower to drive query");
    let (root_hash, _) = drive_query
        .verify_proof_keep_serialized(&case.grovedb_proof, platform_version)
        .unwrap_or_else(|e| panic!("{name}: grovedb layer must verify: {e}"));
    assert_eq!(
        hex::encode(root_hash),
        case.manifest
            .expected_root_hash_hex
            .as_deref()
            .expect("documents cases pin a root hash"),
        "{name}: root hash"
    );

    // Through FromProof the placeholder payload fails document decode cleanly.
    let error = RequestedDocuments::maybe_from_proof_with_metadata(
        request,
        response(&case),
        NETWORK,
        platform_version,
        &case.provider(),
    )
    .expect_err("placeholder document payload must fail to decode");
    assert!(
        matches!(
            error,
            Error::DriveError { .. } | Error::ProtocolError { .. }
        ),
        "{name}: expected a document decode error, got: {error:?}"
    );
}

#[test]
fn dpns_domain_exact_from_wire_request() {
    run_case("dpns-domain-exact");
}

#[test]
fn dpns_domain_prefix_from_wire_request() {
    run_case("dpns-domain-prefix");
}

#[test]
fn dashpay_profile_from_wire_request() {
    run_case("dashpay-profile");
}

#[test]
fn dashpay_contacts_incoming_from_wire_request() {
    run_case("dashpay-contacts-incoming");
}

/// A v0 request carrying the same query as CBOR decodes to the same drive
/// query, so both wire generations verify against one proof.
#[test]
fn v0_cbor_request_lowers_to_the_same_query() {
    let case = load_case("dpns-domain-exact");
    let platform_version = case.platform_version();
    let contract =
        load_system_data_contract(SystemDataContract::DPNS, platform_version).expect("dpns");
    let RequestSpec::DocumentsDpnsExact {
        normalized_label,
        limit,
    } = &case.manifest.request
    else {
        panic!("expected dpns exact");
    };
    let where_value = Value::Array(vec![
        Value::Array(vec![
            Value::Text("normalizedParentDomainName".into()),
            Value::Text("==".into()),
            Value::Text("dash".into()),
        ]),
        Value::Array(vec![
            Value::Text("normalizedLabel".into()),
            Value::Text("==".into()),
            Value::Text(normalized_label.clone()),
        ]),
    ]);
    let request = GetDocumentsRequest {
        version: Some(Version::V0(GetDocumentsRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "domain".to_string(),
            r#where: {
                let mut bytes = Vec::new();
                ciborium::ser::into_writer(&where_value, &mut bytes).expect("cbor");
                bytes
            },
            order_by: vec![],
            limit: u32::from(*limit),
            prove: true,
            start: None,
        })),
    };
    let wire_query = <DocumentWireQuery as drive_proof_verifier::from_request::TryFromRequest<
        GetDocumentsRequest,
    >>::try_from_request(request)
    .expect("decode v0 request");
    let drive_query = wire_query
        .to_drive_query(&contract, None, platform_version)
        .expect("lower");
    let (root_hash, _) = drive_query
        .verify_proof_keep_serialized(&case.grovedb_proof, platform_version)
        .expect("grovedb layer must verify");
    assert_eq!(
        hex::encode(root_hash),
        case.manifest.expected_root_hash_hex.as_deref().unwrap()
    );
}

fn v1(mutate: impl FnOnce(&mut GetDocumentsRequestV1)) -> GetDocumentsRequest {
    let case = load_case("dpns-domain-exact");
    let contract =
        load_system_data_contract(SystemDataContract::DPNS, case.platform_version()).expect("dpns");
    let mut request = v1_request(&case, contract.id().to_vec());
    let Some(Version::V1(inner)) = request.version.as_mut() else {
        unreachable!()
    };
    mutate(inner);
    request
}

fn assert_refused(request: GetDocumentsRequest, needle: &str) {
    let case = load_case("dpns-domain-exact");
    let error = RequestedDocuments::maybe_from_proof_with_metadata(
        request,
        response(&case),
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .expect_err("request shape must be refused");
    match &error {
        Error::RequestError { error } => assert!(
            error.contains(needle),
            "expected a rejection mentioning {needle:?}, got: {error}"
        ),
        other => panic!("expected RequestError, got {other:?}"),
    }
}

#[test]
fn refuses_unproved_request() {
    assert_refused(v1(|r| r.prove = false), "prove=false");
}

#[test]
fn refuses_aggregate_projection() {
    assert_refused(
        v1(|r| {
            r.selects = vec![ProtoSelect {
                function: select::Function::Count as i32,
                field: String::new(),
            }]
        }),
        "aggregate projection",
    );
}

#[test]
fn refuses_group_by_having_and_offset() {
    assert_refused(v1(|r| r.group_by = vec!["label".into()]), "GROUP BY");
    assert_refused(
        v1(|r| {
            r.having = vec![
                dapi_grpc::platform::v0::get_documents_request::HavingClause {
                    aggregate: None,
                    operator: 0,
                    right: None,
                },
            ]
        }),
        "HAVING",
    );
    assert_refused(v1(|r| r.offset = Some(3)), "OFFSET");
}

#[test]
fn refuses_limit_above_server_cap() {
    let case = load_case("dpns-domain-exact");
    let error = RequestedDocuments::maybe_from_proof_with_metadata(
        v1(|r| r.limit = Some(101)),
        response(&case),
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
    .expect_err("limit above the server cap must be refused");
    assert!(
        matches!(error, Error::DriveError { .. }),
        "expected the server's InvalidLimit as a drive error, got {error:?}"
    );
}

#[test]
fn refuses_contract_the_provider_does_not_know() {
    assert_refused(
        v1(|r| r.data_contract_id = vec![9u8; 32]),
        "no data contract",
    );
}

#[test]
fn refuses_v1_limit_zero_but_normalises_v0_zero_to_default() {
    assert_refused(v1(|r| r.limit = Some(0)), "limit = 0");

    let case = load_case("dpns-domain-exact");
    let contract =
        load_system_data_contract(SystemDataContract::DPNS, case.platform_version()).expect("dpns");
    let request = GetDocumentsRequest {
        version: Some(Version::V0(GetDocumentsRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "domain".to_string(),
            r#where: vec![],
            order_by: vec![],
            limit: 0,
            prove: true,
            start: None,
        })),
    };
    let wire_query = <DocumentWireQuery as drive_proof_verifier::from_request::TryFromRequest<
        GetDocumentsRequest,
    >>::try_from_request(request)
    .expect("v0 decodes");
    assert_eq!(wire_query.limit, None, "v0 limit 0 is the unset sentinel");
    let drive_query = wire_query
        .to_drive_query(&contract, None, case.platform_version())
        .expect("lower");
    assert_eq!(
        drive_query.limit,
        Some(100),
        "unset lowers to the server default"
    );
}

#[test]
fn refuses_chained_requests() {
    assert_refused(
        v1(|r| {
            r.chained = Some(Default::default());
        }),
        "chained",
    );
}

#[test]
fn cursors_keep_the_server_inclusion_semantics() {
    let case = load_case("dpns-domain-exact");
    let contract =
        load_system_data_contract(SystemDataContract::DPNS, case.platform_version()).expect("dpns");
    let decode = |request: GetDocumentsRequest| {
        <DocumentWireQuery as drive_proof_verifier::from_request::TryFromRequest<
            GetDocumentsRequest,
        >>::try_from_request(request)
    };
    let none = decode(v1(|_| {})).expect("no cursor");
    assert_eq!((none.start_at, none.start_at_included), (None, true));
    let at = decode(v1(|r| r.start = Some(V1Start::StartAt(vec![7u8; 32])))).expect("start_at");
    assert_eq!((at.start_at, at.start_at_included), (Some([7u8; 32]), true));
    let after =
        decode(v1(|r| r.start = Some(V1Start::StartAfter(vec![8u8; 32])))).expect("start_after");
    assert_eq!(
        (after.start_at, after.start_at_included),
        (Some([8u8; 32]), false)
    );
    decode(v1(|r| r.start = Some(V1Start::StartAt(vec![1u8; 31]))))
        .expect_err("a cursor must be 32 bytes");
    let _ = contract;
}

#[test]
fn refuses_wire_versions_the_platform_version_does_not_serve() {
    // Protocol version 1 predates the v1 getDocuments wire entirely.
    let case = load_case("dpns-domain-exact");
    let contract =
        load_system_data_contract(SystemDataContract::DPNS, case.platform_version()).expect("dpns");
    let request = v1_request(&case, contract.id().to_vec());
    let old = dpp::version::PlatformVersion::get(1).expect("protocol version 1");
    let error = RequestedDocuments::maybe_from_proof_with_metadata(
        request,
        response(&case),
        NETWORK,
        old,
        &case.provider(),
    )
    .expect_err("v1 wire is not served at protocol version 1");
    match &error {
        Error::RequestError { error } => assert!(error.contains("wire version"), "{error}"),
        other => panic!("expected RequestError, got {other:?}"),
    }
}
