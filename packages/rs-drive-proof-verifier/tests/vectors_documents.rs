//! Documents-family (DPNS / Dashpay) proof-vector regression tests.
//!
//! The fixture grovedb state stores placeholder ASCII payloads at the
//! document positions (real documents were not re-fixtured for this corpus),
//! so neither half of these vectors can run a full end-to-end positive
//! verification the way the identity/contested families do. Instead they
//! pin two narrower, still-real things:
//!
//! 1. the [`DriveDocumentQuery`] shape each DAPI query maps to still matches
//!    the proof the server generated. This is checked with `drive`'s own
//!    `verify_proof_keep_serialized` -- a lower-level call on the `drive`
//!    dev-dependency, *not* this crate's public [`FromProof`] entry point --
//!    comparing the root hash and the recovered serialized payloads
//!    byte-for-byte. A DAPI query-shape drift (wrong field name, missing
//!    clause, wrong ordering) changes the query's grovedb path and fails
//!    this root-hash pin.
//! 2. that this crate's real public entry point,
//!    `<Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata`,
//!    itself replays the same grovedb proof internally (via
//!    `DriveDocumentQuery::verify_proof`) and then fails cleanly (an `Err`,
//!    never a panic) once it tries to decode the placeholder payload as a
//!    DPP document -- before the quorum signature is ever consulted.
//!
//! Neither half reaches the tenderdash BLS check: the placeholder payloads
//! make that unreachable here. Coverage of the signature layer itself lives
//! in `vectors_quorum_sig.rs`.

#![cfg(feature = "mocks")]

mod common;

use std::collections::BTreeMap;

use common::{hex_vec, identifier, load_case, Case, Expected, RequestSpec, NETWORK};
use dapi_grpc::platform::v0::{self as platform, get_documents_response};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use drive::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use drive_proof_verifier::{Error, FromProof};
use indexmap::IndexMap;

fn equal_clause(field: &str, value: Value) -> (String, WhereClause) {
    (
        field.to_string(),
        WhereClause {
            field: field.to_string(),
            operator: WhereOperator::Equal,
            value,
        },
    )
}

fn asc_order(field: &str) -> IndexMap<String, OrderClause> {
    let mut order_by = IndexMap::new();
    order_by.insert(
        field.to_string(),
        OrderClause {
            field: field.to_string(),
            ascending: true,
        },
    );
    order_by
}

/// Builds the [`DriveDocumentQuery`] for the case, mirroring the query
/// shapes the Dash Core Platform GUI transport sends (pinned by the fixture
/// vectors).
fn document_query<'a>(case: &Case, contract: &'a DataContract) -> DriveDocumentQuery<'a> {
    let (document_type_name, internal_clauses, order_by, limit) = match &case.manifest.request {
        RequestSpec::DocumentsDpnsExact {
            normalized_label,
            limit,
        } => (
            "domain",
            InternalClauses {
                equal_clauses: BTreeMap::from([
                    equal_clause(
                        "normalizedParentDomainName",
                        Value::Text("dash".to_string()),
                    ),
                    equal_clause("normalizedLabel", Value::Text(normalized_label.to_string())),
                ]),
                ..Default::default()
            },
            IndexMap::new(),
            *limit,
        ),
        RequestSpec::DocumentsDpnsPrefix {
            normalized_prefix,
            limit,
        } => (
            "domain",
            InternalClauses {
                equal_clauses: BTreeMap::from([equal_clause(
                    "normalizedParentDomainName",
                    Value::Text("dash".to_string()),
                )]),
                range_clause: Some(WhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: WhereOperator::StartsWith,
                    value: Value::Text(normalized_prefix.to_string()),
                }),
                ..Default::default()
            },
            asc_order("normalizedLabel"),
            *limit,
        ),
        RequestSpec::DocumentsDashpayProfile { owner_id } => (
            "profile",
            InternalClauses {
                equal_clauses: BTreeMap::from([equal_clause(
                    "$ownerId",
                    Value::Identifier(identifier(owner_id).into_buffer()),
                )]),
                ..Default::default()
            },
            IndexMap::new(),
            1,
        ),
        RequestSpec::DocumentsDashpayContacts {
            identity_id,
            to_identity,
            limit,
        } => (
            "contactRequest",
            InternalClauses {
                equal_clauses: BTreeMap::from([equal_clause(
                    if *to_identity { "toUserId" } else { "$ownerId" },
                    Value::Identifier(identifier(identity_id).into_buffer()),
                )]),
                ..Default::default()
            },
            asc_order("$createdAt"),
            *limit,
        ),
        _ => panic!("{}: not a documents request", case.name),
    };
    DriveDocumentQuery {
        contract,
        document_type: contract
            .document_type_for_name(document_type_name)
            .expect("system contract document type"),
        internal_clauses,
        offset: None,
        limit: Some(limit),
        order_by,
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
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

fn run_documents_case(name: &str) {
    let case = load_case(name);
    let Expected::DocumentsPlaceholder {
        serialized_documents,
    } = &case.manifest.expected
    else {
        panic!("{name}: manifest expectation mismatch");
    };
    let platform_version = case.platform_version();
    let contract = load_system_data_contract(contract_for(&case), platform_version)
        .expect("load system data contract");
    let query = document_query(&case, &contract);

    // The grovedb layer must verify: same root hash the fixture quorum
    // signed, and exactly the recorded placeholder payloads.
    let (root_hash, serialized) = query
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
    assert_eq!(
        serialized,
        serialized_documents
            .iter()
            .map(|hex| hex_vec(hex))
            .collect::<Vec<_>>(),
        "{name}: proven serialized payloads"
    );

    // Through the crate's public API the placeholder payload cannot decode
    // into a DPP document: FromProof must surface a clean error.
    let response = platform::GetDocumentsResponse {
        version: Some(get_documents_response::Version::V0(
            get_documents_response::GetDocumentsResponseV0 {
                metadata: Some(case.metadata()),
                result: Some(
                    get_documents_response::get_documents_response_v0::Result::Proof(
                        case.grpc_proof(),
                    ),
                ),
            },
        )),
    };
    let error = <Documents as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
        query,
        response,
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
fn dpns_domain_exact() {
    run_documents_case("dpns-domain-exact");
}

#[test]
fn dpns_domain_prefix() {
    run_documents_case("dpns-domain-prefix");
}

#[test]
fn dashpay_profile() {
    run_documents_case("dashpay-profile");
}

#[test]
fn dashpay_contacts_incoming() {
    run_documents_case("dashpay-contacts-incoming");
}
