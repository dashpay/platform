// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Acceptance tests for the FromProof-driven (request bytes, response bytes)
//! verification seam: synthesize the DAPI protobuf request/response pairs the
//! C++ transport exchanges around the proof-vector corpus, push the corpus
//! quorum key through the provider store, and verify end to end (grovedb
//! replay + Tenderdash BLS quorum signature).
//!
//! The corpus stores placeholder payloads at document positions, so document
//! queries pin a clean decode failure (as the upstream corpus does) while the
//! identity and contested-vote families verify positively.

mod common;

use common::{hex_vec, load_case, setup, Case, QUORUM_TYPE};
use dapi_grpc::platform::v0::{self as proto, Proof, ResponseMetadata};
use dash_platform_cxx::verify;
use prost::Message;

const DPNS_CONTRACT_ID_HEX: &str =
    "e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c53155";

// --- request/response synthesis used by transport adapters ------------------

fn identity_nonce_request(identity_id: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityNonceRequest {
        version: Some(proto::get_identity_nonce_request::Version::V0(
            proto::get_identity_nonce_request::GetIdentityNonceRequestV0 {
                identity_id,
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn identity_nonce_response(proof: Proof, metadata: ResponseMetadata) -> Vec<u8> {
    proto::GetIdentityNonceResponse {
        version: Some(proto::get_identity_nonce_response::Version::V0(
            proto::get_identity_nonce_response::GetIdentityNonceResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_identity_nonce_response::get_identity_nonce_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

fn identity_contract_nonce_request(identity_id: Vec<u8>, contract_id: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityContractNonceRequest {
        version: Some(proto::get_identity_contract_nonce_request::Version::V0(
            proto::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                identity_id,
                contract_id,
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn identity_contract_nonce_response(proof: Proof, metadata: ResponseMetadata) -> Vec<u8> {
    proto::GetIdentityContractNonceResponse {
        version: Some(proto::get_identity_contract_nonce_response::Version::V0(
            proto::get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

fn identity_request(id: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityRequest {
        version: Some(proto::get_identity_request::Version::V0(
            proto::get_identity_request::GetIdentityRequestV0 { id, prove: true },
        )),
    }
    .encode_to_vec()
}

fn identity_response(proof: Proof, metadata: ResponseMetadata) -> Vec<u8> {
    proto::GetIdentityResponse {
        version: Some(proto::get_identity_response::Version::V0(
            proto::get_identity_response::GetIdentityResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_identity_response::get_identity_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

/// bincode (standard config) encoding of a platform Value::Text, as
/// drive-abci decodes contested index values (discriminant 18 + length +
/// utf8; labels are short so the length is a single byte).
fn bincode_text(text: &str) -> Vec<u8> {
    let mut out = vec![18u8, u8::try_from(text.len()).expect("short label")];
    out.extend_from_slice(text.as_bytes());
    out
}

fn contested_request(case: &Case) -> Vec<u8> {
    let request = &case.manifest.request;
    let index_values: Vec<Vec<u8>> = request["index_values"]
        .as_array()
        .expect("index_values")
        .iter()
        .map(|value| bincode_text(value.as_str().expect("text index value")))
        .collect();
    proto::GetContestedResourceVoteStateRequest {
        version: Some(proto::get_contested_resource_vote_state_request::Version::V0(
            proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
                contract_id: hex_vec(request["contract_id"].as_str().expect("contract_id")),
                document_type_name: request["document_type_name"].as_str().expect("type").to_string(),
                index_name: request["index_name"].as_str().expect("index").to_string(),
                index_values,
                result_type:
                    proto::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType::VoteTally
                        .into(),
                allow_include_locked_and_abstaining_vote_tally: true,
                start_at_identifier_info: None,
                count: Some(request["count"].as_u64().expect("count") as u32),
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn contested_response(proof: Proof, metadata: ResponseMetadata) -> Vec<u8> {
    proto::GetContestedResourceVoteStateResponse {
        version: Some(proto::get_contested_resource_vote_state_response::Version::V0(
            proto::get_contested_resource_vote_state_response::GetContestedResourceVoteStateResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

/// CBOR where clauses exactly as the C++ transport encodes them
/// (transport/cbor.h): definite-length arrays of [field, operator, value].
fn dpns_exact_documents_request(contract_id: Vec<u8>, normalized_label: &str) -> Vec<u8> {
    let clauses = vec![
        vec![
            ciborium::Value::Text("normalizedParentDomainName".to_string()),
            ciborium::Value::Text("==".to_string()),
            ciborium::Value::Text("dash".to_string()),
        ],
        vec![
            ciborium::Value::Text("normalizedLabel".to_string()),
            ciborium::Value::Text("==".to_string()),
            ciborium::Value::Text(normalized_label.to_string()),
        ],
    ];
    let cbor = ciborium::Value::Array(
        clauses
            .into_iter()
            .map(ciborium::Value::Array)
            .collect::<Vec<_>>(),
    );
    let mut where_bytes = Vec::new();
    ciborium::into_writer(&cbor, &mut where_bytes).expect("encode where clauses");
    proto::GetDocumentsRequest {
        version: Some(proto::get_documents_request::Version::V0(
            proto::get_documents_request::GetDocumentsRequestV0 {
                data_contract_id: contract_id,
                document_type: "domain".to_string(),
                r#where: where_bytes,
                order_by: Vec::new(),
                limit: 1,
                prove: true,
                start: None,
            },
        )),
    }
    .encode_to_vec()
}

fn documents_response(proof: Proof, metadata: ResponseMetadata) -> Vec<u8> {
    proto::GetDocumentsResponse {
        version: Some(proto::get_documents_response::Version::V0(
            proto::get_documents_response::GetDocumentsResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    proto::get_documents_response::get_documents_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

// --- positive cases ---------------------------------------------------------

#[test]
fn identity_nonce_verifies_end_to_end() {
    setup();
    let case = load_case("identity-nonce");
    let request = identity_nonce_request(case.identity_id());
    let response = identity_nonce_response(case.proof(), case.metadata());
    let (nonce, meta) =
        verify::verify_get_identity_nonce(&request, &response).expect("identity nonce verifies");
    assert_eq!(nonce, case.manifest.expected["nonce"].as_u64());
    case.check_meta(&meta);
}

#[test]
fn identity_contract_nonce_verifies_end_to_end() {
    setup();
    let case = load_case("identity-contract-nonce");
    let contract_id = hex_vec(
        case.manifest.request["contract_id"]
            .as_str()
            .expect("contract"),
    );
    let request = identity_contract_nonce_request(case.identity_id(), contract_id);
    let response = identity_contract_nonce_response(case.proof(), case.metadata());
    let (nonce, meta) = verify::verify_get_identity_contract_nonce(&request, &response)
        .expect("identity contract nonce verifies");
    assert_eq!(nonce, case.manifest.expected["nonce"].as_u64());
    case.check_meta(&meta);
}

#[test]
fn contested_vote_state_active_verifies_end_to_end() {
    setup();
    let case = load_case("contested-vote-state-active");
    let (state, meta) = verify::verify_get_contested_vote_state(
        &contested_request(&case),
        &contested_response(case.proof(), case.metadata()),
    )
    .expect("contested vote state verifies");
    case.check_meta(&meta);
    assert!(state.contest_found);
    assert!(!state.finished);
    let expected = case.manifest.expected["contenders"].as_array().unwrap();
    assert_eq!(state.contenders.len(), expected.len());
    for ((identity, votes), expected) in state.contenders.iter().zip(expected) {
        assert_eq!(
            hex::encode(identity),
            expected["identity_id"].as_str().unwrap()
        );
        assert_eq!(votes.map(u64::from), expected["votes"].as_u64());
    }
    assert_eq!(
        state.abstain_votes.map(u64::from),
        case.manifest.expected["abstain_votes"].as_u64()
    );
    assert_eq!(
        state.lock_votes.map(u64::from),
        case.manifest.expected["lock_votes"].as_u64()
    );
}

#[test]
fn contested_vote_state_finished_verifies_end_to_end() {
    setup();
    let case = load_case("contested-vote-state-finished");
    let (state, _) = verify::verify_get_contested_vote_state(
        &contested_request(&case),
        &contested_response(case.proof(), case.metadata()),
    )
    .expect("contested vote state verifies");
    assert!(state.contest_found);
    assert!(state.finished);
    assert!(!state.locked);
    assert_eq!(
        state.winner.map(hex::encode),
        case.manifest.expected["winner_identity_id"]
            .as_str()
            .map(String::from)
    );
    assert!(state.finished_at_time_ms > 0);
}

#[test]
fn contested_vote_state_absent_is_proven() {
    setup();
    let case = load_case("contested-vote-state-absent");
    let (state, _) = verify::verify_get_contested_vote_state(
        &contested_request(&case),
        &contested_response(case.proof(), case.metadata()),
    )
    .expect("contested vote state verifies");
    assert!(!state.contest_found);
    assert!(state.contenders.is_empty());
}

/// The corpus grovedb state stores placeholder payloads at document
/// positions: the grovedb + query-shape verification succeeds, and the
/// document decode must fail cleanly (an Err, never an abort).
#[test]
fn placeholder_documents_fail_decoding_cleanly() {
    setup();
    let case = load_case("dpns-domain-exact");
    let request = dpns_exact_documents_request(hex_vec(DPNS_CONTRACT_ID_HEX), "alice");
    let response = documents_response(case.proof(), case.metadata());
    let err = verify::verify_get_documents(&request, &response)
        .expect_err("placeholder documents must not decode");
    // A query-shape drift would surface as a grovedb proof error instead.
    assert!(
        !err.contains("grovedb"),
        "unexpected proof-layer error: {err}"
    );
}

// --- negative cases: quorum binding -----------------------------------------

fn nonce_case_with(mutate: impl FnOnce(&mut Proof, &mut ResponseMetadata)) -> Result<(), String> {
    setup();
    let case = load_case("identity-nonce");
    let mut proof = case.proof();
    let mut metadata = case.metadata();
    mutate(&mut proof, &mut metadata);
    verify::verify_get_identity_nonce(
        &identity_nonce_request(case.identity_id()),
        &identity_nonce_response(proof, metadata),
    )
    .map(|_| ())
}

#[test]
fn tampered_signature_is_rejected() {
    nonce_case_with(|proof, _| proof.signature[10] ^= 0x01).expect_err("tampered signature");
}

#[test]
fn unknown_quorum_hash_is_rejected() {
    let err = nonce_case_with(|proof, _| proof.quorum_hash[0] ^= 0x01).expect_err("unknown quorum");
    assert!(err.contains("quorum"), "unexpected error: {err}");
}

/// A proof naming a quorum type other than the network's Platform type is
/// refused before any key is looked up, whatever keys the embedder pushed.
#[test]
fn non_platform_quorum_type_is_rejected() {
    let err = nonce_case_with(|proof, _| proof.quorum_type = QUORUM_TYPE + 1)
        .expect_err("non-platform quorum type");
    assert!(
        err.contains("quorum type"),
        "expected the quorum-type gate, got: {err}"
    );
}

#[test]
fn tampered_block_id_hash_is_rejected() {
    nonce_case_with(|proof, _| proof.block_id_hash[0] ^= 0x01).expect_err("tampered block id");
}

#[test]
fn tampered_grovedb_proof_is_rejected() {
    nonce_case_with(|proof, _| {
        let mid = proof.grovedb_proof.len() / 2;
        proof.grovedb_proof[mid] ^= 0x01;
    })
    .expect_err("tampered grovedb proof");
}

/// Every signed metadata field is part of the quorum-signature preimage: a
/// replayed proof with any one altered must fail even though the signature
/// is valid for the original block.
#[test]
fn tampered_signed_metadata_is_rejected() {
    nonce_case_with(|_, mtd| mtd.height += 1).expect_err("height");
    nonce_case_with(|_, mtd| mtd.time_ms += 1).expect_err("time_ms");
    nonce_case_with(|_, mtd| mtd.core_chain_locked_height += 1).expect_err("core height");
    nonce_case_with(|_, mtd| mtd.chain_id.push('x')).expect_err("chain id");
    nonce_case_with(|proof, _| proof.round += 1).expect_err("round");
}

/// A response claiming a protocol version this build does not know cannot be
/// verified; it must be refused, not verified under a guessed version.
#[test]
fn unknown_protocol_version_is_rejected() {
    let err = nonce_case_with(|_, mtd| mtd.protocol_version = u32::MAX)
        .expect_err("unknown protocol version");
    assert!(err.contains("protocol version"), "unexpected error: {err}");
}

// --- negative cases: request binding ----------------------------------------

/// A proof for identity A presented with a request for identity B must fail.
#[test]
fn proof_for_a_different_identity_is_rejected() {
    setup();
    let case = load_case("identity-nonce");
    let mut other = case.identity_id();
    other[0] ^= 0x01;
    verify::verify_get_identity_nonce(
        &identity_nonce_request(other),
        &identity_nonce_response(case.proof(), case.metadata()),
    )
    .expect_err("request/proof identity mismatch");
}

/// A structurally valid proof for a different query shape must fail cleanly
/// when presented as a full-identity proof.
#[test]
fn identity_rejects_wrong_shape_proof() {
    setup();
    let case = load_case("identity-nonce");
    let err = verify::verify_get_identity(
        &identity_request(case.identity_id()),
        &identity_response(case.proof(), case.metadata()),
    )
    .expect_err("nonce-shaped proof must not satisfy full-identity verification");
    assert!(err.contains("identity proof verification failed"), "{err}");
}

// --- input hygiene ----------------------------------------------------------

#[test]
fn oversized_messages_are_refused_before_decoding() {
    setup();
    let huge = vec![0u8; verify::MAX_MESSAGE_BYTES + 1];
    let err = verify::verify_get_identity_nonce(&huge, &[]).expect_err("oversized request");
    assert!(err.contains("above the"), "{err}");
    let err = verify::verify_get_identity_nonce(&[], &huge).expect_err("oversized response");
    assert!(err.contains("above the"), "{err}");
}

#[test]
fn garbage_and_empty_inputs_fail_cleanly() {
    setup();
    for bytes in [&b""[..], &[0xffu8; 64][..], b"not protobuf at all"] {
        verify::verify_get_identity_nonce(bytes, bytes).expect_err("garbage input");
        verify::verify_get_documents(bytes, bytes).expect_err("garbage input");
        verify::verify_get_contested_vote_state(bytes, bytes).expect_err("garbage input");
    }
}
