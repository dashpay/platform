// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Acceptance tests for the FromProof-driven (request bytes, response bytes)
//! verification seam: synthesize the DAPI protobuf request/response pairs
//! the C++ transport exchanges — grovedb proofs and the quorum signature
//! come from the package fixture corpus — push the
//! fixture quorum key through the provider store, and verify end to end
//! (grovedb replay + Tenderdash BLS quorum signature).
//!
//! Every fixture proof commits to the same root hash and the fixture quorum
//! signed exactly that root, so all positive cases run the full pipeline.
//! The fixture grovedb state stores placeholder payloads at document
//! positions; document queries therefore pin a clean decode failure (the
//! upstream corpus pins the same), while the identity and contested-vote
//! families verify positively.

use std::sync::Once;

use dapi_grpc::platform::v0::{self as proto, Proof, ResponseMetadata};
use prost::Message;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct DriveVectorFile {
    identity_id: String,
    queries: Vec<QueryVector>,
}

#[derive(Deserialize)]
struct QueryVector {
    name: String,
    #[serde(default)]
    query: Value,
    grovedb_proof_hex: String,
    expected: Value,
}

#[derive(Deserialize)]
struct QuorumVectorFile {
    vectors: Vec<QuorumVector>,
}

#[derive(Deserialize)]
struct QuorumVector {
    name: String,
    envelope: Envelope,
    quorum_public_key: String,
    block_context: BlockContext,
}

#[derive(Deserialize)]
struct Envelope {
    quorum_type: u32,
    quorum_hash: String,
    block_id_hash: String,
    round: u32,
    signature: String,
}

#[derive(Deserialize)]
struct BlockContext {
    height: u64,
    core_chain_locked_height: u32,
    time_ms: u64,
    protocol_version: u32,
    chain_id: String,
}

fn hexv(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("bad hex in test vector")
}

fn drive_vectors() -> DriveVectorFile {
    serde_json::from_str(include_str!("../test_data/drive_query_vectors.json"))
        .expect("parse drive_query_vectors.json")
}

fn quorum_valid() -> QuorumVector {
    let file: QuorumVectorFile =
        serde_json::from_str(include_str!("../test_data/quorum_sig_vectors.json"))
            .expect("parse quorum_sig_vectors.json");
    file.vectors
        .into_iter()
        .find(|vector| vector.name == "valid")
        .expect("valid quorum vector")
}

fn query<'a>(file: &'a DriveVectorFile, name: &str) -> &'a QueryVector {
    file.queries
        .iter()
        .find(|query| query.name == name)
        .unwrap_or_else(|| panic!("query vector not found: {name}"))
}

/// Installs the fixture context exactly once per test binary: the network
/// the fixture chain id belongs to and the fixture quorum key. Tests that
/// need a failing key lookup tamper the response's quorum hash instead of
/// mutating this shared store.
fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        dash_platform_cxx::provider::set_context("test", 0).expect("set_context");
        let quorum = quorum_valid();
        dash_platform_cxx::provider::update_quorum_keys(
            quorum.envelope.quorum_type as u8,
            vec![dash_platform_cxx::provider::QuorumKey {
                quorum_hash: hexv(&quorum.envelope.quorum_hash)
                    .try_into()
                    .expect("32-byte quorum hash"),
                public_key: hexv(&quorum.quorum_public_key)
                    .try_into()
                    .expect("48-byte quorum key"),
            }],
        );
    });
}

/// The proof envelope binding `grovedb_proof` to the fixture-signed block.
fn proof_msg(grovedb_proof: Vec<u8>) -> Proof {
    let quorum = quorum_valid();
    Proof {
        grovedb_proof,
        quorum_hash: hexv(&quorum.envelope.quorum_hash),
        signature: hexv(&quorum.envelope.signature),
        round: quorum.envelope.round,
        block_id_hash: hexv(&quorum.envelope.block_id_hash),
        quorum_type: quorum.envelope.quorum_type,
    }
}

fn metadata() -> ResponseMetadata {
    let ctx = quorum_valid().block_context;
    ResponseMetadata {
        height: ctx.height,
        core_chain_locked_height: ctx.core_chain_locked_height,
        epoch: 0,
        time_ms: ctx.time_ms,
        protocol_version: ctx.protocol_version,
        chain_id: ctx.chain_id,
    }
}

fn check_meta(meta: &dash_platform_cxx::types::Meta) {
    let ctx = quorum_valid().block_context;
    assert_eq!(meta.height, ctx.height);
    assert_eq!(meta.core_chain_locked_height, ctx.core_chain_locked_height);
    assert_eq!(meta.time_ms, ctx.time_ms);
    assert_eq!(meta.protocol_version, ctx.protocol_version);
    assert_eq!(meta.chain_id, ctx.chain_id);
}

// --- request/response synthesis used by transport adapters

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

fn identity_nonce_response(proof: Proof) -> Vec<u8> {
    proto::GetIdentityNonceResponse {
        version: Some(proto::get_identity_nonce_response::Version::V0(
            proto::get_identity_nonce_response::GetIdentityNonceResponseV0 {
                metadata: Some(metadata()),
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

fn identity_contract_nonce_response(proof: Proof) -> Vec<u8> {
    proto::GetIdentityContractNonceResponse {
        version: Some(proto::get_identity_contract_nonce_response::Version::V0(
            proto::get_identity_contract_nonce_response::GetIdentityContractNonceResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_identity_contract_nonce_response::get_identity_contract_nonce_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

fn identity_balance_request(identity_id: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityBalanceRequest {
        version: Some(proto::get_identity_balance_request::Version::V0(
            proto::get_identity_balance_request::GetIdentityBalanceRequestV0 {
                id: identity_id,
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn identity_balance_response(proof: Proof) -> Vec<u8> {
    proto::GetIdentityBalanceResponse {
        version: Some(proto::get_identity_balance_response::Version::V0(
            proto::get_identity_balance_response::GetIdentityBalanceResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_identity_balance_response::get_identity_balance_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

fn identity_keys_request(identity_id: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityKeysRequest {
        version: Some(proto::get_identity_keys_request::Version::V0(
            proto::get_identity_keys_request::GetIdentityKeysRequestV0 {
                identity_id,
                request_type: Some(proto::KeyRequestType {
                    request: Some(proto::key_request_type::Request::AllKeys(proto::AllKeys {})),
                }),
                limit: None,
                offset: None,
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn identity_keys_response(proof: Proof) -> Vec<u8> {
    proto::GetIdentityKeysResponse {
        version: Some(proto::get_identity_keys_response::Version::V0(
            proto::get_identity_keys_response::GetIdentityKeysResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_identity_keys_response::get_identity_keys_response_v0::Result::Proof(
                        proof,
                    ),
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

fn contested_request(contract_id: Vec<u8>, normalized_label: &str, count: u32) -> Vec<u8> {
    proto::GetContestedResourceVoteStateRequest {
        version: Some(proto::get_contested_resource_vote_state_request::Version::V0(
            proto::get_contested_resource_vote_state_request::GetContestedResourceVoteStateRequestV0 {
                contract_id,
                document_type_name: "domain".to_string(),
                index_name: "parentNameAndLabel".to_string(),
                index_values: vec![bincode_text("dash"), bincode_text(normalized_label)],
                result_type:
                    proto::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType::VoteTally
                        .into(),
                allow_include_locked_and_abstaining_vote_tally: true,
                start_at_identifier_info: None,
                count: Some(count),
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn contested_response(proof: Proof) -> Vec<u8> {
    proto::GetContestedResourceVoteStateResponse {
        version: Some(proto::get_contested_resource_vote_state_response::Version::V0(
            proto::get_contested_resource_vote_state_response::GetContestedResourceVoteStateResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

/// CBOR where/order-by clauses exactly as the C++ transport encodes them
/// (transport/cbor.h): definite-length arrays of [field, operator, value]
/// triples.
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

fn documents_response(proof: Proof) -> Vec<u8> {
    proto::GetDocumentsResponse {
        version: Some(proto::get_documents_response::Version::V0(
            proto::get_documents_response::GetDocumentsResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_documents_response::get_documents_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

const DPNS_CONTRACT_ID_HEX: &str =
    "e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c53155";

// --- positive cases --------------------------------------------------------

#[test]
fn identity_nonce_verifies_end_to_end() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_nonce");
    let request = identity_nonce_request(hexv(&file.identity_id));
    let response = identity_nonce_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (nonce, meta) = dash_platform_cxx::verify::verify_get_identity_nonce(&request, &response)
        .expect("identity nonce verifies");
    assert_eq!(nonce, q.expected["nonce"].as_u64());
    check_meta(&meta);
}

#[test]
fn identity_contract_nonce_verifies_end_to_end() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_contract_nonce");
    let contract_id = hexv(q.query["contract_id"].as_str().expect("contract_id"));
    let request = identity_contract_nonce_request(hexv(&file.identity_id), contract_id);
    let response = identity_contract_nonce_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (nonce, meta) =
        dash_platform_cxx::verify::verify_get_identity_contract_nonce(&request, &response)
            .expect("identity contract nonce verifies");
    assert_eq!(nonce, q.expected["nonce"].as_u64());
    check_meta(&meta);
}

#[test]
fn identity_balance_verifies_end_to_end() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_balance");
    let request = identity_balance_request(hexv(&file.identity_id));
    let response = identity_balance_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (balance, meta) =
        dash_platform_cxx::verify::verify_get_identity_balance(&request, &response)
            .expect("identity balance verifies");
    assert_eq!(balance, q.expected["balance"].as_u64());
    check_meta(&meta);
}

#[test]
fn identity_keys_verify_end_to_end() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_keys");
    let request = identity_keys_request(hexv(&file.identity_id));
    let response = identity_keys_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (keys, meta) = dash_platform_cxx::verify::verify_get_identity_keys(&request, &response)
        .expect("identity keys verify");
    let keys = keys.expect("identity proven present");
    assert_eq!(
        keys.iter().map(|key| key.id).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        keys.len(),
        q.expected["serialized_keys"]
            .as_array()
            .expect("keys")
            .len()
    );
    check_meta(&meta);
}

#[test]
fn contested_vote_state_active_verifies_end_to_end() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "contested_vote_state_active");
    let label = q.query["normalized_label"].as_str().expect("label");
    let count = q.query["count"].as_u64().expect("count") as u32;
    let request = contested_request(hexv(DPNS_CONTRACT_ID_HEX), label, count);
    let response = contested_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (state, meta) =
        dash_platform_cxx::verify::verify_get_contested_vote_state(&request, &response)
            .expect("contested vote state verifies");
    check_meta(&meta);
    assert!(state.contest_found);
    assert!(!state.finished);
    let expected_contenders = q.expected["contenders"].as_array().unwrap();
    assert_eq!(state.contenders.len(), expected_contenders.len());
    for ((identity, votes), expected) in state.contenders.iter().zip(expected_contenders) {
        assert_eq!(
            hex::encode(identity),
            expected["identity_id"].as_str().unwrap()
        );
        assert_eq!(votes.map(u64::from), expected["votes"].as_u64());
    }
    assert_eq!(
        state.abstain_votes.map(u64::from),
        q.expected["abstain_votes"].as_u64()
    );
    assert_eq!(
        state.lock_votes.map(u64::from),
        q.expected["lock_votes"].as_u64()
    );
}

#[test]
fn contested_vote_state_finished_verifies_end_to_end() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "contested_vote_state_finished");
    let label = q.query["normalized_label"].as_str().expect("label");
    let count = q.query["count"].as_u64().expect("count") as u32;
    let request = contested_request(hexv(DPNS_CONTRACT_ID_HEX), label, count);
    let response = contested_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (state, _) =
        dash_platform_cxx::verify::verify_get_contested_vote_state(&request, &response)
            .expect("contested vote state verifies");
    assert!(state.contest_found);
    assert!(state.finished);
    assert!(!state.locked);
    assert_eq!(
        state.winner.map(hex::encode),
        q.expected["winner_identity_id"].as_str().map(String::from)
    );
    assert!(state.finished_at_time_ms > 0);
}

#[test]
fn contested_vote_state_absent_is_proven() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "contested_vote_state_absent");
    let label = q.query["normalized_label"].as_str().expect("label");
    let count = q.query["count"].as_u64().expect("count") as u32;
    let request = contested_request(hexv(DPNS_CONTRACT_ID_HEX), label, count);
    let response = contested_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let (state, _) =
        dash_platform_cxx::verify::verify_get_contested_vote_state(&request, &response)
            .expect("contested vote state verifies");
    assert!(!state.contest_found);
    assert!(state.contenders.is_empty());
}

// --- decode-failure pin ----------------------------------------------------

/// The fixture grovedb state stores placeholder payloads at document
/// positions: the grovedb + query-shape verification succeeds, and the
/// document decode must fail cleanly (an Err, never a panic). The upstream
/// rs-drive-proof-verifier corpus pins the same behavior.
#[test]
fn placeholder_documents_fail_decoding_cleanly() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "dpns_exact");
    let request = dpns_exact_documents_request(hexv(DPNS_CONTRACT_ID_HEX), "alice");
    let response = documents_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let err = dash_platform_cxx::verify::verify_get_documents(&request, &response)
        .expect_err("placeholder documents must not decode");
    // The failure must come from the DPP decode, not from the grovedb
    // replay: a query-shape drift would surface as a "grovedb:" proof error.
    assert!(
        !err.contains("grovedb"),
        "unexpected proof-layer error: {err}"
    );
}

// --- negative cases (quorum binding) ---------------------------------------

#[test]
fn tampered_signature_is_rejected() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_nonce");
    let mut proof = proof_msg(hexv(&q.grovedb_proof_hex));
    proof.signature[10] ^= 0x01;
    let request = identity_nonce_request(hexv(&file.identity_id));
    let response = identity_nonce_response(proof);
    dash_platform_cxx::verify::verify_get_identity_nonce(&request, &response)
        .expect_err("tampered signature must fail");
}

#[test]
fn unknown_quorum_is_rejected() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_nonce");
    let mut proof = proof_msg(hexv(&q.grovedb_proof_hex));
    proof.quorum_hash[0] ^= 0x01;
    let request = identity_nonce_request(hexv(&file.identity_id));
    let response = identity_nonce_response(proof);
    let err = dash_platform_cxx::verify::verify_get_identity_nonce(&request, &response)
        .expect_err("unknown quorum must fail");
    assert!(err.contains("quorum"), "unexpected error: {err}");
}

#[test]
fn tampered_grovedb_proof_is_rejected() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_nonce");
    let mut grovedb_proof = hexv(&q.grovedb_proof_hex);
    let mid = grovedb_proof.len() / 2;
    grovedb_proof[mid] ^= 0x01;
    let request = identity_nonce_request(hexv(&file.identity_id));
    let response = identity_nonce_response(proof_msg(grovedb_proof));
    dash_platform_cxx::verify::verify_get_identity_nonce(&request, &response)
        .expect_err("tampered grovedb proof must fail");
}

/// The signed metadata is part of the quorum-signature preimage: a replayed
/// proof with altered height must fail even though the signature itself is
/// valid for the original block.
#[test]
fn tampered_metadata_height_is_rejected() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_nonce");
    let request = identity_nonce_request(hexv(&file.identity_id));
    let mut mtd = metadata();
    mtd.height += 1;
    let response = proto::GetIdentityNonceResponse {
        version: Some(proto::get_identity_nonce_response::Version::V0(
            proto::get_identity_nonce_response::GetIdentityNonceResponseV0 {
                metadata: Some(mtd),
                result: Some(
                    proto::get_identity_nonce_response::get_identity_nonce_response_v0::Result::Proof(
                        proof_msg(hexv(&q.grovedb_proof_hex)),
                    ),
                ),
            },
        )),
    }
    .encode_to_vec();
    dash_platform_cxx::verify::verify_get_identity_nonce(&request, &response)
        .expect_err("tampered signed height must fail");
}

fn identity_request(id: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityRequest {
        version: Some(proto::get_identity_request::Version::V0(
            proto::get_identity_request::GetIdentityRequestV0 { id, prove: true },
        )),
    }
    .encode_to_vec()
}

fn identity_response(proof: Proof) -> Vec<u8> {
    proto::GetIdentityResponse {
        version: Some(proto::get_identity_response::Version::V0(
            proto::get_identity_response::GetIdentityResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_identity_response::get_identity_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

fn identity_by_pubkey_hash_request(public_key_hash: Vec<u8>) -> Vec<u8> {
    proto::GetIdentityByPublicKeyHashRequest {
        version: Some(proto::get_identity_by_public_key_hash_request::Version::V0(
            proto::get_identity_by_public_key_hash_request::GetIdentityByPublicKeyHashRequestV0 {
                public_key_hash,
                prove: true,
            },
        )),
    }
    .encode_to_vec()
}

fn identity_by_pubkey_hash_response(proof: Proof) -> Vec<u8> {
    proto::GetIdentityByPublicKeyHashResponse {
        version: Some(proto::get_identity_by_public_key_hash_response::Version::V0(
            proto::get_identity_by_public_key_hash_response::GetIdentityByPublicKeyHashResponseV0 {
                metadata: Some(metadata()),
                result: Some(
                    proto::get_identity_by_public_key_hash_response::get_identity_by_public_key_hash_response_v0::Result::Proof(proof),
                ),
            },
        )),
    }
    .encode_to_vec()
}

/// The fixture corpus has no full-identity proof: the pubkey-hash vectors
/// prove only the unique-hash -> identity-id mapping, while the verifier
/// requires the full identity subtree (upstream skips these vectors for the
/// same reason). Until a full-identity fixture is regenerated, the two
/// flagship identity paths are pinned negatively: an id-mapping-only proof
/// must be rejected cleanly, never verified and never a panic.
#[test]
fn identity_by_pubkey_hash_rejects_id_mapping_only_proof() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_by_public_key_hash_present");
    let request =
        identity_by_pubkey_hash_request(hexv(q.query["public_key_hash"].as_str().unwrap()));
    let response = identity_by_pubkey_hash_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let err = dash_platform_cxx::verify::verify_get_identity_by_pubkey_hash(&request, &response)
        .expect_err("id-mapping-only proof must not satisfy full-identity verification");
    assert!(
        err.contains("identity-by-public-key-hash proof verification failed"),
        "unexpected error: {err}"
    );
}

/// A structurally valid proof for a different query shape must fail cleanly
/// when presented as a full-identity proof.
#[test]
fn identity_rejects_wrong_shape_proof() {
    setup();
    let file = drive_vectors();
    let q = query(&file, "identity_nonce");
    let request = identity_request(hexv(&file.identity_id));
    let response = identity_response(proof_msg(hexv(&q.grovedb_proof_hex)));
    let err = dash_platform_cxx::verify::verify_get_identity(&request, &response)
        .expect_err("nonce-shaped proof must not satisfy full-identity verification");
    assert!(
        err.contains("identity proof verification failed"),
        "unexpected error: {err}"
    );
}
