// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The proved queries, replayed through the same client code an embedder
//! runs, against `dash-sdk`'s mock transport fed with drive-proof-verifier's
//! proof-vector corpus. The mock transport hands the SDK the corpus
//! response bytes; the SDK's own `FromProof` path replays the GroveDB proof
//! and checks the Tenderdash BLS quorum signature against the key the test
//! pushed through the client, exactly as with a live node. Only the socket
//! is mocked.

mod common;

use std::path::PathBuf;

use common::{fixture_context, hex32, hex_vec, mock_client, push_quorum_key, CHAIN_ID};
use dash_platform_cxx::client::{Client, MAX_CORE_CHAINLOCK_LAG};
use dash_platform_cxx::queries;
use dash_sdk::dapi_client::transport::TransportRequest;
use dash_sdk::dapi_client::{DumpData, ExecutionResponse};
use dash_sdk::platform::proto::{self, Proof, ResponseMetadata};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityContractNonceFetcher;
use dash_sdk::{Sdk, SdkBuilder};
use serde::Deserialize;

// --- corpus access ----------------------------------------------------------

#[derive(Deserialize)]
struct Manifest {
    request: serde_json::Value,
    expected: serde_json::Value,
    block: BlockMeta,
    proof_meta: ProofMeta,
}

#[derive(Deserialize)]
struct BlockMeta {
    height: u64,
    core_chain_locked_height: u32,
    epoch: u16,
    time_ms: u64,
    protocol_version: u32,
    chain_id: String,
}

#[derive(Deserialize)]
struct ProofMeta {
    round: u32,
    quorum_type: u32,
    quorum_hash_hex: String,
    block_id_hash_hex: String,
}

struct Case {
    manifest: Manifest,
    grovedb_proof: Vec<u8>,
    signature: Vec<u8>,
    quorum_pubkey: [u8; 48],
}

fn load_case(name: &str) -> Case {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-drive-proof-verifier/tests/vectors")
        .join(name);
    let read = |file: &str| {
        std::fs::read_to_string(dir.join(file))
            .unwrap_or_else(|e| panic!("read corpus file {name}/{file}: {e}"))
    };
    Case {
        manifest: serde_json::from_str(&read("manifest.json")).expect("corpus manifest"),
        grovedb_proof: hex_vec(read("proof.hex").trim()),
        signature: hex_vec(read("signature.hex").trim()),
        quorum_pubkey: hex_vec(read("quorum_pubkey.hex").trim())
            .try_into()
            .expect("48-byte quorum key"),
    }
}

impl Case {
    fn proof(&self) -> Proof {
        Proof {
            grovedb_proof: self.grovedb_proof.clone(),
            quorum_hash: hex_vec(&self.manifest.proof_meta.quorum_hash_hex),
            signature: self.signature.clone(),
            round: self.manifest.proof_meta.round,
            block_id_hash: hex_vec(&self.manifest.proof_meta.block_id_hash_hex),
            quorum_type: self.manifest.proof_meta.quorum_type,
        }
    }

    fn metadata(&self) -> ResponseMetadata {
        ResponseMetadata {
            height: self.manifest.block.height,
            core_chain_locked_height: self.manifest.block.core_chain_locked_height,
            epoch: u32::from(self.manifest.block.epoch),
            time_ms: self.manifest.block.time_ms,
            protocol_version: self.manifest.block.protocol_version,
            chain_id: self.manifest.block.chain_id.clone(),
        }
    }

    fn identity_id(&self) -> Vec<u8> {
        hex_vec(
            self.manifest.request["identity_id"]
                .as_str()
                .expect("identity_id"),
        )
    }

    fn contract_id(&self) -> Vec<u8> {
        hex_vec(
            self.manifest.request["contract_id"]
                .as_str()
                .expect("contract_id"),
        )
    }

    fn quorum_hash(&self) -> [u8; 32] {
        hex32(&self.manifest.proof_meta.quorum_hash_hex)
    }
}

// --- mock transport wiring ---------------------------------------------------

fn nonce_request(case: &Case) -> proto::GetIdentityContractNonceRequest {
    proto::GetIdentityContractNonceRequest {
        version: Some(proto::get_identity_contract_nonce_request::Version::V0(
            proto::get_identity_contract_nonce_request::GetIdentityContractNonceRequestV0 {
                identity_id: case.identity_id(),
                contract_id: case.contract_id(),
                prove: true,
            },
        )),
    }
}

fn nonce_response(
    proof: Proof,
    metadata: ResponseMetadata,
) -> proto::GetIdentityContractNonceResponse {
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
}

/// Writes one canned (request, response) pair into a fresh dump directory
/// the mock SDK loads at build time, and installs that SDK on the client.
fn install_mock_sdk<R>(client: &Client, request: &R, response: R::Response)
where
    R: TransportRequest,
    R::Response: Clone,
{
    // Tests run in parallel; a wall-clock name can collide between threads
    // and hand one test another's expectation.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "dash-platform-cxx-mock-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("dump dir");
    let result = Ok(ExecutionResponse {
        inner: response,
        retries: 0,
        address: "http://127.0.0.1:1".parse().expect("address"),
    });
    let dump = DumpData::new(request, &result);
    dump.save(&dir.join(dump.filename().expect("dump file name")))
        .expect("save mock expectation");
    // Same freshness configuration as `Client::set_endpoints`, seeded from
    // the client's watermark so a rebuilt SDK carries it forward.
    let sdk: Sdk = SdkBuilder::new_mock()
        .with_network(fixture_context().network)
        .with_context_provider(std::sync::Arc::clone(client.provider()))
        .with_height_tolerance(Some(3))
        .with_trusted_initial_height(client.last_seen_height())
        .with_dump_dir(&dir)
        .build()
        .expect("mock sdk");
    // The expectation is in memory now; the dump dir was only its carrier.
    let _ = std::fs::remove_dir_all(&dir);
    client.set_sdk(sdk).expect("install mock sdk");
}

fn nonce_client(case: &Case, metadata: ResponseMetadata) -> Client {
    let client = mock_client();
    push_quorum_key(&client, case.quorum_hash(), case.quorum_pubkey);
    install_mock_sdk(
        &client,
        &nonce_request(case),
        nonce_response(case.proof(), metadata),
    );
    client
}

fn get_nonce(
    client: &Client,
    case: &Case,
) -> Result<(Option<u64>, dash_platform_cxx::types::Meta), String> {
    queries::get_identity_contract_nonce(client, &case.identity_id(), &case.contract_id())
}

// --- tests ------------------------------------------------------------------

#[test]
fn identity_nonce_verifies_through_the_sdk_end_to_end() {
    let case = load_case("identity-contract-nonce");
    let client = nonce_client(&case, case.metadata());
    let (nonce, meta) = get_nonce(&client, &case).expect("nonce verifies");
    assert_eq!(nonce, case.manifest.expected["nonce"].as_u64());
    assert_eq!(meta.height, case.manifest.block.height);
    assert_eq!(
        meta.core_chain_locked_height,
        case.manifest.block.core_chain_locked_height
    );
    assert_eq!(meta.time_ms, case.manifest.block.time_ms);
    assert_eq!(meta.protocol_version, case.manifest.block.protocol_version);
    assert_eq!(meta.chain_id, CHAIN_ID);
}

#[test]
fn unknown_quorum_key_is_rejected_by_the_sdk() {
    let case = load_case("identity-contract-nonce");
    let client = mock_client();
    // No quorum key pushed: the SDK's verifier must fail the lookup.
    install_mock_sdk(
        &client,
        &nonce_request(&case),
        nonce_response(case.proof(), case.metadata()),
    );
    let err = get_nonce(&client, &case).unwrap_err();
    assert!(err.contains("no locally known Platform quorum"), "{err}");
}

#[test]
fn non_platform_quorum_type_is_rejected() {
    let case = load_case("identity-contract-nonce");
    let client = mock_client();
    push_quorum_key(&client, case.quorum_hash(), case.quorum_pubkey);
    let mut proof = case.proof();
    proof.quorum_type += 1;
    install_mock_sdk(
        &client,
        &nonce_request(&case),
        nonce_response(proof, case.metadata()),
    );
    let err = get_nonce(&client, &case).unwrap_err();
    assert!(err.contains("quorum type"), "{err}");
}

#[test]
fn tampered_signature_is_rejected() {
    let case = load_case("identity-contract-nonce");
    let client = mock_client();
    push_quorum_key(&client, case.quorum_hash(), case.quorum_pubkey);
    let mut proof = case.proof();
    proof.signature[10] ^= 0x01;
    install_mock_sdk(
        &client,
        &nonce_request(&case),
        nonce_response(proof, case.metadata()),
    );
    assert!(get_nonce(&client, &case).is_err());
}

#[test]
fn response_signed_for_another_chain_is_rejected() {
    // A validly signed response from a network with a different Tenderdash
    // chain id is refused by the client after the SDK verified it.
    let case = load_case("identity-contract-nonce");
    let client = mock_client();
    let mut other_chain = fixture_context();
    other_chain.tenderdash_chain_id = "dash-mainnet".to_string();
    client.set_context(other_chain).expect("context");
    push_quorum_key(&client, case.quorum_hash(), case.quorum_pubkey);
    install_mock_sdk(
        &client,
        &nonce_request(&case),
        nonce_response(case.proof(), case.metadata()),
    );
    let err = get_nonce(&client, &case).unwrap_err();
    assert!(err.contains("signed for tenderdash chain"), "{err}");
}

#[test]
fn signed_core_height_far_behind_the_local_chainlock_is_stale() {
    let case = load_case("identity-contract-nonce");
    let client = nonce_client(&case, case.metadata());
    let signed = case.manifest.block.core_chain_locked_height;
    // Exactly at the lag bound is still accepted...
    client.set_core_chain_locked_height(signed + MAX_CORE_CHAINLOCK_LAG as u32);
    get_nonce(&client, &case).expect("within the lag bound");
    // ...one block past it is stale.
    let client = nonce_client(&case, case.metadata());
    client.set_core_chain_locked_height(signed + MAX_CORE_CHAINLOCK_LAG as u32 + 1);
    let err = get_nonce(&client, &case).unwrap_err();
    assert!(err.contains("stale platform proof"), "{err}");
}

#[test]
fn verified_height_watermark_survives_a_rebuilt_sdk() {
    let case = load_case("identity-contract-nonce");
    let client = mock_client();
    push_quorum_key(&client, case.quorum_hash(), case.quorum_pubkey);
    // A previously verified response put the watermark well above the
    // corpus height (the corpus has a single signed height, so the earlier
    // response is simulated at the post-verification hook).
    let mut ahead = case.metadata();
    ahead.height += 10;
    client.accept(&ahead).expect("earlier verified response");
    assert_eq!(client.last_seen_height(), ahead.height);
    // A rebuilt SDK (new endpoint set) is seeded with that watermark, so a
    // node serving the older corpus state is refused by the SDK's monotonic
    // height check even though its proof verifies.
    install_mock_sdk(
        &client,
        &nonce_request(&case),
        nonce_response(case.proof(), case.metadata()),
    );
    let err = get_nonce(&client, &case).unwrap_err();
    assert!(err.contains("outdated"), "{err}");
}

#[test]
fn sdk_types_are_reachable_for_direct_rust_callers() {
    // The bridge is the C++ surface; Rust embedders can still drive the
    // SDK directly through the client. Pins that `Client::run` accepts an
    // SDK future and maps its error.
    let case = load_case("identity-contract-nonce");
    let client = nonce_client(&case, case.metadata());
    let id = dash_sdk::platform::Identifier::from_bytes(&case.identity_id()).expect("id");
    let contract = dash_sdk::platform::Identifier::from_bytes(&case.contract_id()).expect("id");
    let (nonce, _) = client
        .run(move |sdk| async move {
            IdentityContractNonceFetcher::fetch_with_metadata(&sdk, (id, contract), None).await
        })
        .expect("direct sdk call");
    assert_eq!(nonce.map(|n| n.0), case.manifest.expected["nonce"].as_u64());
}

#[test]
fn queries_without_endpoints_fail_cleanly() {
    let client = Client::new();
    client.set_context(fixture_context()).expect("context");
    let err = queries::get_identity(&client, &[0u8; 32]).unwrap_err();
    assert!(err.contains("set_endpoints"), "{err}");
    let err = queries::resolve_name(&client, "alice").unwrap_err();
    assert!(err.contains("set_endpoints"), "{err}");
}

#[test]
fn bad_inputs_are_refused_before_any_request() {
    let client = mock_client();
    assert!(queries::get_identity(&client, &[0u8; 31]).is_err());
    assert!(queries::get_identity_by_pubkey_hash(&client, &[0u8; 19]).is_err());
    assert!(queries::get_identity_contract_nonce(&client, &[0u8; 32], &[0u8; 33]).is_err());
    assert!(queries::broadcast_state_transition(&client, &vec![0u8; 100 * 1024 + 1]).is_err());
}

#[test]
fn endpoints_are_validated_and_unchanged_sets_are_ignored() {
    let client = Client::new();
    assert!(
        client
            .set_endpoints(vec!["https://1.2.3.4:1443".into()])
            .is_err(),
        "no context yet"
    );
    client.set_context(fixture_context()).expect("context");
    assert!(client.set_endpoints(Vec::new()).is_err(), "empty set");
    assert!(
        client.set_endpoints(vec!["not a uri".into()]).is_err(),
        "bad uri"
    );
    client
        .set_endpoints(vec!["https://1.2.3.4:1443".into()])
        .expect("valid endpoint");
    client
        .set_endpoints(vec!["https://1.2.3.4:1443".into()])
        .expect("same set is a no-op");
    client.shutdown();
    client.shutdown();
}
