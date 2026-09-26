// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The proved reads and the broadcast, replayed through the same client
//! code an embedder runs, against `dash-sdk`'s mock transport fed with
//! proofs generated from a real Drive state (`common::Fixture`). The SDK's
//! own `FromProof` path replays the GroveDB proof and checks the Tenderdash
//! BLS quorum signature against the key the test pushed through the
//! client; only the socket is mocked. Every test runs against the fixture
//! at the protocol version testnet and mainnet run and at this build's
//! latest.

mod common;

use common::{
    assert_kind, bare_client, contested_request, contested_response, documents_request,
    documents_response, execution_response, identity_by_key_hash_request,
    identity_by_key_hash_response, identity_proto_response, identity_request, install_at_floor,
    install_identity, install_ok, install_refusal, mock_client, mock_client_at, nonce_request,
    nonce_response, now_ms, offline_sdk, paged_label, quorum, Fixture, ABSENT_LABEL, CHAIN_ID,
    CONTESTED_LABEL, CORE_CHAIN_LOCKED_HEIGHT, DEPLOYED_VERSION, DPNS_NONCE, HEIGHT, OWNED_LABEL,
    PAGED_NAME_COUNT, PLATFORM_LLMQ_TYPE,
};
use dash_platform_cxx::client::{Client, HEIGHT_TOLERANCE};
use dash_platform_cxx::ffi::{StatusKind, WinnerKind};
use dash_platform_cxx::ops;
use dash_platform_cxx::provider::MAX_CORE_CHAINLOCK_LAG;
use dash_sdk::dapi_client::mock::MockDapiClient;
use dash_sdk::dapi_client::transport::TransportError;
use dash_sdk::dapi_client::{DapiClientError, ExecutionError};
use dash_sdk::dapi_grpc::tonic::metadata::{MetadataMap, MetadataValue};
use dash_sdk::dapi_grpc::tonic::{Code, Status};
use dash_sdk::dpp::consensus::basic::identity::IdentityAssetLockProofLockedTransactionMismatchError;
use dash_sdk::dpp::consensus::basic::BasicError;
use dash_sdk::dpp::consensus::codes::ErrorWithCode;
use dash_sdk::dpp::consensus::ConsensusError;
use dash_sdk::dpp::dashcore::hashes::Hash;
use dash_sdk::dpp::dashcore::Txid;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
use dash_sdk::dpp::version::v14::PROTOCOL_VERSION_14;
use dash_sdk::dpp::version::{ProtocolVersion, LATEST_VERSION};
use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
use dash_sdk::platform::proto::{self, ResponseMetadata};
use dash_sdk::platform::types::identity::IdentityResponse;
use dash_sdk::platform::DocumentQuery;
use dash_sdk::query_types::IdentityContractNonceFetcher;
use test_case::test_matrix;

/// A client that answers `get_identity(alice)` with the fixture proof under
/// `metadata`.
fn identity_client(fixture: &Fixture, metadata: ResponseMetadata) -> Client {
    let client = mock_client();
    install_identity(fixture, &client, metadata, |_| {});
    client
}

fn alice_id(fixture: &Fixture) -> [u8; 32] {
    fixture.alice.id().to_buffer()
}

// --- identities --------------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn identity_verifies_end_to_end_with_its_keys(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = identity_client(fixture, fixture.metadata());
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.value.id, alice_id(fixture));
    assert_eq!(result.value.balance, fixture.alice.balance());
    assert_eq!(result.value.keys.len(), 4);
    let key2 = &result.value.keys[2];
    assert_eq!((key2.id, key2.purpose, key2.security_level), (2, 1, 3));
    assert_eq!(
        key2.bounds.kind,
        dash_platform_cxx::ffi::BoundsKind::SingleContractDocumentType
    );
    assert_eq!(key2.bounds.contract_id, fixture.dashpay().id().to_buffer());
    assert_eq!(key2.bounds.document_type, "contactRequest");
    assert_eq!(result.meta.height, HEIGHT);
    assert_eq!(
        result.meta.core_chain_locked_height,
        CORE_CHAIN_LOCKED_HEIGHT
    );
    assert_eq!(result.meta.chain_id, CHAIN_ID);
    assert_eq!(
        result.meta.protocol_version,
        fixture.version.protocol_version
    );
    assert_eq!(client.last_seen_height(), HEIGHT);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn identity_by_public_key_hash_verifies(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let metadata = fixture.metadata();
    let proof = fixture.proof(
        fixture.prove_identity_by_key_hash(fixture.alice_key0_hash),
        &metadata,
    );
    install_ok(
        fixture,
        &client,
        &identity_by_key_hash_request(fixture.alice_key0_hash),
        identity_by_key_hash_response(proof, metadata),
    );
    let result = ops::get_identity_by_pubkey_hash(&client, fixture.alice_key0_hash);
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.value.id, alice_id(fixture));
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn unknown_public_key_hash_is_proven_absent(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let metadata = fixture.metadata();
    let hash = [0x77u8; 20];
    let proof = fixture.proof(fixture.prove_identity_by_key_hash(hash), &metadata);
    install_ok(
        fixture,
        &client,
        &identity_by_key_hash_request(hash),
        identity_by_key_hash_response(proof, metadata),
    );
    let result = ops::get_identity_by_pubkey_hash(&client, hash);
    assert_kind(&result.status, StatusKind::ProvenAbsent);
    assert_eq!(
        result.meta.height, HEIGHT,
        "absence still carries verified metadata"
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn identity_contract_nonce_is_masked(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let metadata = fixture.metadata();
    let contract = fixture.dpns().id();
    let proof = fixture.proof(
        fixture.prove_identity_contract_nonce(fixture.alice.id(), contract),
        &metadata,
    );
    install_ok(
        fixture,
        &client,
        &nonce_request(fixture.alice.id(), contract),
        nonce_response(proof, metadata),
    );
    // The stored nonce carries missing-revision bits above the 40-bit
    // value: the SDK's own reads see them, the shell masks them.
    let raw = client
        .run(move |sdk| async move {
            let query = (fixture.alice.id(), contract);
            dash_sdk::platform::Fetch::fetch_with_metadata(&sdk, query, None)
                .await
                .map(|(fetcher, _): (Option<IdentityContractNonceFetcher>, _)| fetcher.map(|f| f.0))
        })
        .expect("direct fetch")
        .expect("fetch")
        .expect("present");
    assert_eq!(raw & IDENTITY_NONCE_VALUE_FILTER, DPNS_NONCE);
    assert_ne!(
        raw, DPNS_NONCE,
        "the stored nonce carries missing-revision bits"
    );
    let result = ops::get_identity_contract_nonce(&client, alice_id(fixture), contract.to_buffer());
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.value, DPNS_NONCE);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn nonce_of_an_unused_contract_is_proven_absent(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let metadata = fixture.metadata();
    let contract = fixture.dashpay().id();
    let proof = fixture.proof(
        fixture.prove_identity_contract_nonce(fixture.alice.id(), contract),
        &metadata,
    );
    install_ok(
        fixture,
        &client,
        &nonce_request(fixture.alice.id(), contract),
        nonce_response(proof, metadata),
    );
    let result = ops::get_identity_contract_nonce(&client, alice_id(fixture), contract.to_buffer());
    assert_kind(&result.status, StatusKind::ProvenAbsent);
    assert_eq!(
        result.value, 0,
        "the next nonce is 1, as after a value of 0"
    );
    assert_eq!(result.meta.height, HEIGHT);
}

// --- freshness and trust gates -----------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn proofless_response_is_rejected(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let response = IdentityResponse::GetIdentity(identity_proto_response(
        proto::get_identity_response::get_identity_response_v0::Result::Identity(vec![]),
        fixture.metadata(),
    ));
    install_ok(
        fixture,
        &client,
        &identity_request(fixture.alice.id()),
        response,
    );
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Rejected);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn tampered_grovedb_proof_is_rejected(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let metadata = fixture.metadata();
    let mut grovedb_proof = fixture.prove_identity(fixture.alice.id());
    let middle = grovedb_proof.len() / 2;
    grovedb_proof[middle] ^= 0x01;
    let proof = fixture.proof(grovedb_proof, &metadata);
    install_ok(
        fixture,
        &client,
        &identity_request(fixture.alice.id()),
        common::identity_response(proof, metadata),
    );
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Rejected,
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn tampered_quorum_signature_is_rejected(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    install_identity(fixture, &client, fixture.metadata(), |proof| {
        proof.signature[10] ^= 0x01
    });
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Rejected,
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn unknown_quorum_is_rejected(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = identity_client(fixture, fixture.metadata());
    client.provider().set_quorum_keys(&[]);
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_eq!(result.status.kind, StatusKind::Rejected);
    assert!(result
        .status
        .message
        .contains("no locally known Platform quorum"));
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn quorum_hash_in_proof_order_does_not_match(protocol_version: ProtocolVersion) {
    // The embedder pushes its internal byte order; a caller that already
    // reversed the hash pushes the wrong key.
    let fixture = Fixture::get(protocol_version);
    let client = identity_client(fixture, fixture.metadata());
    client
        .provider()
        .set_quorum_keys(&[dash_platform_cxx::ffi::QuorumKey {
            hash: quorum().hash,
            pubkey: quorum().pubkey,
        }]);
    assert_eq!(
        ops::get_identity(&client, alice_id(fixture)).status.kind,
        StatusKind::Rejected
    );
    client
        .provider()
        .set_quorum_keys(&[quorum().core_order_key()]);
    assert_eq!(
        ops::get_identity(&client, alice_id(fixture)).status.kind,
        StatusKind::Ok
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn non_platform_quorum_type_is_rejected(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    install_identity(fixture, &client, fixture.metadata(), |proof| {
        proof.quorum_type = u32::from(PLATFORM_LLMQ_TYPE) + 1
    });
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_eq!(result.status.kind, StatusKind::Rejected);
    assert!(result.status.message.contains("quorum type"));
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn foreign_chain_id_is_a_mismatch_and_moves_no_shell_state(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let mut metadata = fixture.metadata();
    metadata.chain_id = "dash-mainnet".to_string();
    let client = identity_client(fixture, metadata);
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::ChainIdMismatch);
    assert_eq!(
        client.last_seen_height(),
        0,
        "the shell watermark is untouched"
    );
    // The same signed proof under the configured chain id verifies.
    let client = identity_client(fixture, fixture.metadata());
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Ok,
    );
    assert_eq!(client.last_seen_height(), HEIGHT);
    assert!(client.verified_platform_version().is_ok());
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn foreign_chain_id_still_ratchets_the_sdk_version(protocol_version: ProtocolVersion) {
    // The SDK ratchets its protocol version inside proof verification,
    // before the shell's chain-id check runs: a validly signed proof from
    // another chain of a higher known version moves the SDK version (from
    // the floor to 14; at 13 there is nothing to move). The shell keeps its
    // own state untouched (above) and P1 (`with_expected_chain_id`) moves
    // the check upstream; this pins the current order so a change is
    // noticed.
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let mut metadata = fixture.metadata();
    metadata.chain_id = "dash-mainnet".to_string();
    let proof = fixture.proof(fixture.prove_identity(fixture.alice.id()), &metadata);
    install_at_floor(
        &client,
        &identity_request(fixture.alice.id()),
        common::identity_response(proof, metadata),
    );
    assert_eq!(client.platform_version().protocol_version, DEPLOYED_VERSION);
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::ChainIdMismatch);
    assert_eq!(
        client.platform_version().protocol_version,
        fixture.version.protocol_version,
        "the SDK ratcheted on the foreign chain's verified metadata"
    );
    assert_eq!(client.last_seen_height(), 0, "the shell moved nothing");
    assert!(
        client.verified_platform_version().is_err(),
        "no transition can be built before a read the shell accepted"
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn no_local_anchor_is_unavailable_and_dispatches_nothing(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = bare_client();
    // No expectation is installed: the read must not reach the transport.
    client.set_sdk(offline_sdk(&client, fixture.version));
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Unavailable);
    assert!(result.status.message.contains("anchor"));
    client
        .provider()
        .set_local_core_chain_locked_height(CORE_CHAIN_LOCKED_HEIGHT);
    install_identity(fixture, &client, fixture.metadata(), |_| {});
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Ok,
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn version_ratchets_up_from_the_network_floor_on_a_verified_response(
    protocol_version: ProtocolVersion,
) {
    // A network SDK seeds itself at the per-network floor (PV13 on
    // testnet) and learns the real version from the first verified
    // response, the fixture's. Until then no transition can be built, not
    // even when the network turns out to run the floor itself.
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let metadata = fixture.metadata();
    let proof = fixture.proof(fixture.prove_identity(fixture.alice.id()), &metadata);
    install_at_floor(
        &client,
        &identity_request(fixture.alice.id()),
        common::identity_response(proof, metadata),
    );
    assert_eq!(client.platform_version().protocol_version, DEPLOYED_VERSION);
    let error = client.verified_platform_version().unwrap_err();
    assert!(error.contains("no verified read yet"), "{error}");
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Ok,
    );
    assert_eq!(
        client.platform_version().protocol_version,
        fixture.version.protocol_version,
        "the SDK follows the verified response"
    );
    assert_eq!(
        client
            .verified_platform_version()
            .expect("verified")
            .protocol_version,
        fixture.version.protocol_version
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn chainlock_lag_floor_and_no_ceiling(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = identity_client(fixture, fixture.metadata());
    client
        .provider()
        .set_local_core_chain_locked_height(CORE_CHAIN_LOCKED_HEIGHT + MAX_CORE_CHAINLOCK_LAG);
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Ok,
    );
    let client = identity_client(fixture, fixture.metadata());
    client
        .provider()
        .set_local_core_chain_locked_height(CORE_CHAIN_LOCKED_HEIGHT + MAX_CORE_CHAINLOCK_LAG + 1);
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Rejected);
    assert!(result.status.message.contains("stale"));
    // A proof one ChainLock ahead of the local node is honest.
    let client_behind = mock_client_at(CORE_CHAIN_LOCKED_HEIGHT - 1);
    install_identity(fixture, &client_behind, fixture.metadata(), |_| {});
    assert_kind(
        &ops::get_identity(&client_behind, alice_id(fixture)).status,
        StatusKind::Ok,
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn platform_height_watermark_tolerates_three_blocks(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let serve = |height: u64| {
        let mut metadata = fixture.metadata();
        metadata.height = height;
        install_identity(fixture, &client, metadata, |_| {});
        ops::get_identity(&client, alice_id(fixture)).status
    };
    assert_kind(&serve(HEIGHT), StatusKind::Ok);
    assert_kind(&serve(HEIGHT - 2), StatusKind::Ok);
    assert_kind(&serve(HEIGHT - HEIGHT_TOLERANCE), StatusKind::Ok);
    let stale = serve(HEIGHT - HEIGHT_TOLERANCE - 1);
    assert_kind(&stale, StatusKind::Rejected);
    assert!(stale.message.contains("trails"));
    assert_kind(&serve(HEIGHT - HEIGHT_TOLERANCE - 2), StatusKind::Rejected);
    assert_eq!(client.last_seen_height(), HEIGHT);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn a_stale_response_records_no_protocol_version(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let serve = |height: u64, protocol_version: u32| {
        let mut metadata = fixture.metadata();
        metadata.height = height;
        metadata.protocol_version = protocol_version;
        install_identity(fixture, &client, metadata, |_| {});
        ops::get_identity(&client, alice_id(fixture)).status
    };
    assert_kind(
        &serve(HEIGHT, fixture.version.protocol_version),
        StatusKind::Ok,
    );
    let verified = client
        .verified_platform_version()
        .expect("verified")
        .protocol_version;
    // A validly signed but stale response claiming a newer version is
    // rejected before its version is recorded: the builders stay open.
    assert_kind(
        &serve(HEIGHT - HEIGHT_TOLERANCE - 1, LATEST_VERSION + 1),
        StatusKind::Rejected,
    );
    assert_eq!(
        client
            .verified_platform_version()
            .expect("still verified")
            .protocol_version,
        verified
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn signed_time_outside_the_window_is_rejected(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let mut metadata = fixture.metadata();
    metadata.time_ms = now_ms() - 11 * 60 * 1000;
    let client = identity_client(fixture, metadata);
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Rejected);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn unsupported_protocol_version_is_signalled_with_the_value(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let mut metadata = fixture.metadata();
    metadata.protocol_version = LATEST_VERSION + 1;
    let client = identity_client(fixture, metadata);
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::UnsupportedProtocolVersion);
    assert_eq!(
        result.value.id,
        alice_id(fixture),
        "the verified value is still returned"
    );
    assert_eq!(result.meta.protocol_version, LATEST_VERSION + 1);
    assert_eq!(
        client.sdk().expect("sdk").version().protocol_version,
        fixture.version.protocol_version,
        "the SDK never ratchets to an unknown version"
    );
    // The shell records it: no transition is built for a network this
    // build does not know, even after a read at a known version.
    let error = client.verified_platform_version().unwrap_err();
    assert!(error.contains("no transition can be built"), "{error}");
    install_identity(fixture, &client, fixture.metadata(), |_| {});
    assert_kind(
        &ops::get_identity(&client, alice_id(fixture)).status,
        StatusKind::Ok,
    );
    assert!(client.verified_platform_version().is_err());
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn absence_under_an_unsupported_version_is_still_proven_absent(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let mut metadata = fixture.metadata();
    metadata.protocol_version = LATEST_VERSION + 1;
    let hash = [0x77u8; 20];
    let proof = fixture.proof(fixture.prove_identity_by_key_hash(hash), &metadata);
    install_ok(
        fixture,
        &client,
        &identity_by_key_hash_request(hash),
        identity_by_key_hash_response(proof, metadata),
    );
    let result = ops::get_identity_by_pubkey_hash(&client, hash);
    assert_kind(&result.status, StatusKind::ProvenAbsent);
    assert_eq!(result.meta.protocol_version, LATEST_VERSION + 1);
    // The version is recorded all the same: nothing gets built.
    assert!(client.verified_platform_version().is_err());
}

// --- documents ---------------------------------------------------------------

fn dpns_domain_query(fixture: &Fixture) -> DocumentQuery {
    DocumentQuery::new(fixture.dpns().clone(), "domain").expect("query")
}

fn dash_tld_query(fixture: &Fixture) -> DocumentQuery {
    dpns_domain_query(fixture).with_where(WhereClause {
        field: "normalizedParentDomainName".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Text("dash".to_string()),
    })
}

fn install_documents(fixture: &Fixture, client: &Client, query: &DocumentQuery) {
    let metadata = fixture.metadata();
    let proof = fixture.proof(fixture.prove_documents(query), &metadata);
    install_ok(
        fixture,
        client,
        &documents_request(fixture, query),
        documents_response(proof, metadata),
    );
}

/// The continuation of `query` after the document `cursor`.
fn continuation(mut query: DocumentQuery, cursor: [u8; 32]) -> DocumentQuery {
    query.start = Some(
        proto::get_documents_request::get_documents_request_v0::Start::StartAfter(cursor.to_vec()),
    );
    query
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn resolve_name_finds_the_owned_name(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let normalized = dash_platform_cxx::helpers::normalize_label(OWNED_LABEL);
    let query = dash_tld_query(fixture)
        .with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(normalized.clone()),
        })
        .with_limit(1);
    install_documents(fixture, &client, &query);
    let result = ops::resolve_name(&client, OWNED_LABEL);
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.value.label, OWNED_LABEL);
    assert_eq!(result.value.normalized_label, normalized);
    assert_eq!(result.value.parent, "dash");
    assert_eq!(result.value.identity, alice_id(fixture));
    assert_eq!(result.value.owner, alice_id(fixture));
    assert_ne!(result.value.document_id, [0u8; 32]);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn resolve_name_proves_absence(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let query = dash_tld_query(fixture)
        .with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text(dash_platform_cxx::helpers::normalize_label(ABSENT_LABEL)),
        })
        .with_limit(1);
    install_documents(fixture, &client, &query);
    let result = ops::resolve_name(&client, ABSENT_LABEL);
    assert_kind(&result.status, StatusKind::ProvenAbsent);
    assert_eq!(result.meta.height, HEIGHT);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn search_names_by_prefix_clamps_the_limit(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let query = dash_tld_query(fixture)
        .with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::StartsWith,
            value: Value::Text("a1".to_string()),
        })
        .with_order_by(OrderClause {
            field: "normalizedLabel".to_string(),
            ascending: true,
        })
        .with_limit(ops::PAGE_SIZE);
    install_documents(fixture, &client, &query);
    // 500 is clamped to the page size, so the shell issues the query above.
    let result = ops::search_names(&client, "A1", 500, None);
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].label, OWNED_LABEL);
    assert!(!result.page.has_more);
}

#[test]
fn search_names_refuses_what_the_network_would_before_dispatch() {
    // No SDK at all: a dispatched read would come back `Unavailable`.
    let client = mock_client();
    for prefix in ["", &"a".repeat(64)] {
        let result = ops::search_names(&client, prefix, 10, None);
        assert_kind(&result.status, StatusKind::Internal);
        assert!(
            result.status.message.contains("prefix"),
            "{}",
            result.status.message
        );
    }
    assert_kind(
        &ops::search_names(&client, &"a".repeat(63), 10, None).status,
        StatusKind::Unavailable,
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn search_names_pages_against_its_limit_with_a_cursor(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let search = |limit: u32| {
        dash_tld_query(fixture)
            .with_where(WhereClause {
                field: "normalizedLabel".to_string(),
                operator: WhereOperator::StartsWith,
                value: Value::Text("pg2".to_string()),
            })
            .with_order_by(OrderClause {
                field: "normalizedLabel".to_string(),
                ascending: true,
            })
            .with_limit(limit)
    };
    // A page that fills a limit below the page size has more.
    install_documents(fixture, &client, &search(1));
    let one = ops::search_names(&client, "pg2", 1, None);
    assert_kind(&one.status, StatusKind::Ok);
    assert_eq!(one.items.len(), 1);
    assert_eq!(one.items[0].label, paged_label(0));
    assert!(one.page.has_more);
    // The next single row follows the cursor.
    install_documents(
        fixture,
        &client,
        &continuation(search(1), one.page.next_start_after),
    );
    let two = ops::search_names(&client, "pg2", 1, Some(one.page.next_start_after));
    assert_kind(&two.status, StatusKind::Ok);
    assert_eq!(two.items.len(), 1);
    assert_eq!(two.items[0].label, paged_label(1));
    assert!(two.page.has_more);

    install_documents(fixture, &client, &search(ops::PAGE_SIZE));
    let first = ops::search_names(&client, "pg2", ops::PAGE_SIZE, None);
    assert_kind(&first.status, StatusKind::Ok);
    assert_eq!(first.items.len(), ops::PAGE_SIZE as usize);
    assert!(first.page.has_more);
    let cursor = first.page.next_start_after;
    assert_eq!(cursor, first.items.last().expect("an item").document_id);

    install_documents(
        fixture,
        &client,
        &continuation(search(ops::PAGE_SIZE), cursor),
    );
    let rest = ops::search_names(&client, "pg2", ops::PAGE_SIZE, Some(cursor));
    assert_kind(&rest.status, StatusKind::Ok);
    assert_eq!(rest.items.len(), PAGED_NAME_COUNT - ops::PAGE_SIZE as usize);
    assert!(!rest.page.has_more);
    let mut labels: Vec<String> = first
        .items
        .iter()
        .chain(rest.items.iter())
        .map(|name| name.label.clone())
        .collect();
    labels.sort();
    labels.dedup();
    assert_eq!(labels.len(), PAGED_NAME_COUNT);
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn a_read_the_node_refuses_is_rejected_not_unavailable(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let query = dash_tld_query(fixture)
        .with_where(WhereClause {
            field: "normalizedLabel".to_string(),
            operator: WhereOperator::StartsWith,
            value: Value::Text("a1".to_string()),
        })
        .with_order_by(OrderClause {
            field: "normalizedLabel".to_string(),
            ascending: true,
        })
        .with_limit(ops::PAGE_SIZE);
    let request = documents_request(fixture, &query);
    // Drive refuses the query (`Status::invalid_argument`): the node
    // answered, and retrying elsewhere would not change its mind.
    install_refusal(
        fixture,
        &client,
        &request,
        Status::invalid_argument("bad query"),
    );
    let result = ops::search_names(&client, "A1", ops::PAGE_SIZE, None);
    assert_kind(&result.status, StatusKind::Rejected);
    // tonic's answer to a response above the decoding bound.
    install_refusal(
        fixture,
        &client,
        &request,
        Status::out_of_range("too large"),
    );
    assert_kind(
        &ops::search_names(&client, "A1", ops::PAGE_SIZE, None).status,
        StatusKind::Rejected,
    );
    // The node did not answer: an outage.
    install_refusal(fixture, &client, &request, Status::unavailable("down"));
    assert_kind(
        &ops::search_names(&client, "A1", ops::PAGE_SIZE, None).status,
        StatusKind::Unavailable,
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn names_of_identity_pages_with_a_cursor(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let query = dpns_domain_query(fixture)
        .with_where(WhereClause {
            field: "records.identity".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(alice_id(fixture)),
        })
        .with_limit(ops::PAGE_SIZE);
    install_documents(fixture, &client, &query);
    let result = ops::names_of_identity(&client, alice_id(fixture), None);
    assert_kind(&result.status, StatusKind::Ok);
    // Only the owned name: Alice's contender document for the contested
    // name is not in the identity index until the contest resolves.
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].label, OWNED_LABEL);
    assert_eq!(
        result.page.next_start_after,
        result.items.last().expect("an item").document_id
    );
    assert!(!result.page.has_more, "a short page is the last one");
    // A continuation is a distinct query with the cursor.
    install_documents(
        fixture,
        &client,
        &continuation(query, result.page.next_start_after),
    );
    let rest = ops::names_of_identity(
        &client,
        alice_id(fixture),
        Some(result.page.next_start_after),
    );
    if protocol_version < PROTOCOL_VERSION_14 {
        // Protocol version 13 cannot continue this index (see below).
        assert_kind(&rest.status, StatusKind::Unavailable);
        return;
    }
    assert_kind(&rest.status, StatusKind::Ok);
    assert!(rest.items.is_empty());
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn a_full_page_reports_more_and_the_cursor_continues(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let carol = fixture.carol.id().to_buffer();
    let query = dpns_domain_query(fixture)
        .with_where(WhereClause {
            field: "records.identity".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(carol),
        })
        .with_limit(ops::PAGE_SIZE);
    install_documents(fixture, &client, &query);
    let first = ops::names_of_identity(&client, carol, None);
    assert_kind(&first.status, StatusKind::Ok);
    assert_eq!(first.items.len(), ops::PAGE_SIZE as usize);
    assert!(first.page.has_more, "a full page may have more");
    let cursor = first.page.next_start_after;
    assert_eq!(cursor, first.items.last().expect("an item").document_id);

    install_documents(fixture, &client, &continuation(query, cursor));
    let rest = ops::names_of_identity(&client, carol, Some(cursor));
    if protocol_version < PROTOCOL_VERSION_14 {
        // Protocol version 13's lowering of a cursor on this one-property,
        // non-unique index skips every remaining name under the identity:
        // Drive proves an empty last page (fixed in 14), which the shell
        // refuses rather than report the list as complete.
        assert_kind(&rest.status, StatusKind::Unavailable);
        assert!(rest.items.is_empty());
        assert_eq!(rest.meta.protocol_version, protocol_version);
        return;
    }
    assert_kind(&rest.status, StatusKind::Ok);
    assert!(!rest.page.has_more);
    assert_eq!(rest.items.len(), PAGED_NAME_COUNT - ops::PAGE_SIZE as usize);
    // The two pages partition the names: no overlap, nothing skipped.
    let mut labels: Vec<String> = first
        .items
        .iter()
        .chain(rest.items.iter())
        .map(|name| name.label.clone())
        .collect();
    labels.sort();
    labels.dedup();
    assert_eq!(labels.len(), PAGED_NAME_COUNT);
    assert!((0..PAGED_NAME_COUNT).all(|i| labels.contains(&paged_label(i))));
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn profile_verifies_and_absence_is_proven(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let profile_query = |owner: [u8; 32]| {
        DocumentQuery::new(fixture.dashpay().clone(), "profile")
            .expect("query")
            .with_where(WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(owner),
            })
            .with_limit(1)
    };
    install_documents(fixture, &client, &profile_query(alice_id(fixture)));
    let result = ops::get_profile(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.value.display_name, "Alice");
    assert_eq!(result.value.public_message, "hello");
    assert_eq!(result.value.revision, 1);
    assert_eq!(result.value.owner, alice_id(fixture));
    assert!(result.value.avatar_url.is_empty());
    assert_eq!(
        result.value.core_payment_address,
        fixture.core_payment_address
    );
    assert!(result.value.platform_payment_address.is_empty());
    assert_ne!(result.value.created_at, 0);

    let bob = fixture.bob.id().to_buffer();
    install_documents(fixture, &client, &profile_query(bob));
    let absent = ops::get_profile(&client, bob);
    assert_kind(&absent.status, StatusKind::ProvenAbsent);
    assert_eq!(
        absent.meta.height, HEIGHT,
        "absence still carries verified metadata"
    );
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn profile_reads_at_the_network_floor_before_the_first_ratchet(protocol_version: ProtocolVersion) {
    // A network SDK's first proved read runs at the per-network protocol
    // floor. At 14 the fixture profile is stored under DashPay v2, which
    // the floor's contract cannot decode; at 13 under v1. The shell
    // queries with the latest compiled-in contract, which decodes both.
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let query = DocumentQuery::new(fixture.dashpay().clone(), "profile")
        .expect("query")
        .with_where(WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(alice_id(fixture)),
        })
        .with_limit(1);
    let metadata = fixture.metadata();
    let proof = fixture.proof(fixture.prove_documents(&query), &metadata);
    install_at_floor(
        &client,
        &documents_request(fixture, &query),
        documents_response(proof, metadata),
    );
    assert_eq!(client.platform_version().protocol_version, DEPLOYED_VERSION);
    let result = ops::get_profile(&client, alice_id(fixture));
    assert_kind(&result.status, StatusKind::Ok);
    assert_eq!(result.value.display_name, "Alice");
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn contact_requests_to_me_and_from_me(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let bob = fixture.bob.id().to_buffer();
    let contact_query = |field: &str, id: [u8; 32], since: Option<u64>| {
        let mut query = DocumentQuery::new(fixture.dashpay().clone(), "contactRequest")
            .expect("query")
            .with_where(WhereClause {
                field: field.to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(id),
            });
        if let Some(since) = since {
            query = query.with_where(WhereClause {
                field: "$createdAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::U64(since),
            });
        }
        query
            .with_order_by(OrderClause {
                field: "$createdAt".to_string(),
                ascending: true,
            })
            .with_limit(ops::PAGE_SIZE)
    };
    install_documents(fixture, &client, &contact_query("toUserId", bob, None));
    let to_bob = ops::get_contact_requests(&client, bob, true, 0, None);
    assert_kind(&to_bob.status, StatusKind::Ok);
    assert_eq!(to_bob.items.len(), 1);
    let request = &to_bob.items[0];
    assert_eq!(request.owner, alice_id(fixture));
    assert_eq!(request.to_user_id, bob);
    assert_eq!(request.encrypted_public_key.len(), 96);
    assert_eq!(
        (request.sender_key_index, request.recipient_key_index),
        (2, 3)
    );
    assert_eq!(request.account_reference, 0x1000_0005);
    assert!(request.encrypted_account_label.is_empty());
    assert_eq!(request.core_height_created_at, CORE_CHAIN_LOCKED_HEIGHT);
    assert!(!to_bob.page.has_more);

    install_documents(
        fixture,
        &client,
        &contact_query("$ownerId", alice_id(fixture), Some(1_700_000_000_000)),
    );
    let from_alice_later =
        ops::get_contact_requests(&client, alice_id(fixture), false, 1_700_000_000_000, None);
    assert_kind(&from_alice_later.status, StatusKind::Ok);
    assert!(
        from_alice_later.items.is_empty(),
        "nothing after the creation time"
    );
}

// --- contested names ---------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn contested_vote_state_tallies_and_absence(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let install_contest = |client: &Client, label: &str| {
        let query = fixture.contested_query(&dash_platform_cxx::helpers::normalize_label(label));
        let metadata = fixture.metadata();
        let proof = fixture.proof(fixture.prove_contested_vote_state(query.clone()), &metadata);
        install_ok(
            fixture,
            client,
            &contested_request(fixture, &query),
            contested_response(proof, metadata),
        );
    };
    install_contest(&client, CONTESTED_LABEL);
    let result = ops::get_contested_vote_state(&client, CONTESTED_LABEL);
    assert_kind(&result.status, StatusKind::Ok);
    let state = result.value;
    assert_eq!(state.winner_kind, WinnerKind::NoWinner);
    assert_eq!(state.ends_at, 0);
    assert_eq!((state.abstain, state.lock), (1, 4));
    assert_eq!(state.contenders.len(), 2);
    let votes = |id: [u8; 32]| {
        state
            .contenders
            .iter()
            .find(|contender| contender.identity == id)
            .map(|contender| (contender.votes, contender.has_votes))
    };
    assert_eq!(votes(alice_id(fixture)), Some((5, true)));
    assert_eq!(votes(fixture.bob.id().to_buffer()), Some((1, true)));

    install_contest(&client, ABSENT_LABEL);
    let absent = ops::get_contested_vote_state(&client, ABSENT_LABEL);
    assert_kind(&absent.status, StatusKind::ProvenAbsent);
}

// --- broadcast ---------------------------------------------------------------

fn broadcast_request(bytes: &[u8]) -> proto::BroadcastStateTransitionRequest {
    proto::BroadcastStateTransitionRequest {
        state_transition: bytes.to_vec(),
    }
}

fn grpc_failure(status: Status) -> ExecutionError<DapiClientError> {
    ExecutionError {
        inner: DapiClientError::Transport(TransportError::Grpc(status)),
        retries: 0,
        address: None,
    }
}

/// Submits `bytes` through a mock transport answering with `result`, the
/// same executor path `Sdk::execute` takes on a mock SDK.
fn submit_with(
    bytes: &[u8],
    result: Result<(), ExecutionError<DapiClientError>>,
) -> dash_platform_cxx::ffi::Status {
    let request = broadcast_request(bytes);
    let mut transport = MockDapiClient::new();
    let result = result.map(|()| execution_response(proto::BroadcastStateTransitionResponse {}));
    transport.expect(&request, &result).expect("expectation");
    futures::executor::block_on(ops::submit(&transport, request))
}

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn broadcast_outcomes_are_typed(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let bytes = vec![0x02u8; 64];

    assert_eq!(submit_with(&bytes, Ok(())).kind, StatusKind::Ok);
    assert_eq!(
        submit_with(
            &bytes,
            Err(grpc_failure(Status::new(
                Code::AlreadyExists,
                "already in mempool"
            )))
        )
        .kind,
        StatusKind::AlreadyExists
    );
    // A definitive non-consensus refusal is a rejection, an outage is not.
    assert_eq!(
        submit_with(
            &bytes,
            Err(grpc_failure(Status::new(
                Code::InvalidArgument,
                "malformed"
            )))
        )
        .kind,
        StatusKind::Rejected
    );
    assert_eq!(
        submit_with(
            &bytes,
            Err(grpc_failure(Status::new(Code::Unavailable, "down")))
        )
        .kind,
        StatusKind::Unavailable
    );

    // A consensus error travels as gRPC metadata; the SDK's error
    // conversion decodes it and the shell reports its code. (The mock
    // transport serializes a gRPC status without its metadata, so this
    // outcome is checked on the classifier directly.)
    let consensus_error = ConsensusError::BasicError(
        BasicError::IdentityAssetLockProofLockedTransactionMismatchError(
            IdentityAssetLockProofLockedTransactionMismatchError::new(
                Txid::from_byte_array([0; 32]),
                Txid::from_byte_array([1; 32]),
            ),
        ),
    );
    let mut metadata = MetadataMap::new();
    metadata.insert_bin(
        "dash-serialized-consensus-error-bin",
        MetadataValue::from_bytes(
            &consensus_error
                .serialize_to_bytes_with_platform_version(fixture.version)
                .expect("serialize"),
        ),
    );
    let status = ops::broadcast_status(Err(DapiClientError::Transport(TransportError::Grpc(
        Status::with_metadata(Code::InvalidArgument, "consensus", metadata),
    ))));
    assert_kind(&status, StatusKind::Consensus);
    assert_eq!(status.consensus_code, consensus_error.code());
    assert_eq!(
        ops::broadcast_status(Err(DapiClientError::NoAvailableAddresses)).kind,
        StatusKind::Unavailable
    );

    // Through the client: the size bound, and a transport that never
    // answers is an outage, not a success.
    let client = mock_client();
    client.set_sdk(offline_sdk(&client, fixture.version));
    assert_eq!(
        ops::broadcast(&client, &vec![0u8; 100 * 1024 + 1])
            .status
            .kind,
        StatusKind::Internal
    );
    let unanswered = ops::broadcast(&client, &bytes).status;
    assert_ne!(unanswered.kind, StatusKind::Ok);
    assert_ne!(unanswered.kind, StatusKind::AlreadyExists);
}

#[test]
fn broadcast_to_an_unreachable_endpoint_is_unavailable() {
    let client = Client::new(&common::config()).expect("client");
    client
        .set_endpoints(&["https://127.0.0.1:1".to_string()])
        .expect("endpoint");
    let result = ops::broadcast(&client, &[0x02u8; 64]);
    assert_kind(&result.status, StatusKind::Unavailable);
    client.shutdown();
}

// --- lifecycle ---------------------------------------------------------------

#[test_matrix([DEPLOYED_VERSION, LATEST_VERSION])]
fn reads_without_endpoints_or_after_shutdown_are_unavailable(protocol_version: ProtocolVersion) {
    let fixture = Fixture::get(protocol_version);
    let client = mock_client();
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_eq!(result.status.kind, StatusKind::Unavailable);
    assert!(result.status.message.contains("no evonode endpoints"));
    let client = identity_client(fixture, fixture.metadata());
    assert_eq!(
        ops::get_identity(&client, alice_id(fixture)).status.kind,
        StatusKind::Ok
    );
    client.shutdown();
    let result = ops::get_identity(&client, alice_id(fixture));
    assert_eq!(result.status.kind, StatusKind::Unavailable);
    assert!(
        result.status.message.contains("shut down"),
        "{}",
        result.status.message
    );
    client.shutdown();
}
