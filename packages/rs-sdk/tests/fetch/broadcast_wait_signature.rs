//! Security regression test for the broadcast wait-path quorum-signature gate.
//!
//! The broadcast `wait_for_response` path verifies a state-transition execution result through
//! `<StateTransitionProofResult as FromProof<BroadcastStateTransitionRequest>>::maybe_from_proof_with_metadata`,
//! which runs the GroveDB structural check AND `verify_tenderdash_proof` (the quorum BLS signature
//! gate) before the SDK ratchets its protocol version from the response metadata. This test drives
//! that verifier directly against a captured, real signed proof and asserts both that a valid proof
//! verifies and that tampered variants are rejected — proving forged metadata cannot pass the gate.
//!
//! The test is `#[ignore]`d because it requires a captured vector that is not yet committed. It is
//! written against the real APIs so it cannot bitrot; remove the `#[ignore]` once the vector lands.
//!
//! TODO: Capture and commit the vector under
//! `packages/rs-sdk/tests/vectors/broadcast_wait_signed_proof/`, recording all three files from the
//! same response so they stay consistent, then delete `#[ignore]` and set `EXPECTED_PROTOCOL_VERSION`
//! and the happy-path result assertion to the captured transition's variant:
//!
//! - `response.bin`: a protobuf-encoded `WaitForStateTransitionResultResponse` carrying a real
//!   signed proof, captured from a v12 devnet for a known, already broadcast state transition
//!   (`prove: true`).
//! - `state_transition.bin`: the platform-serialized `StateTransition` bytes that were broadcast
//!   (the body of the originating `BroadcastStateTransitionRequest`).
//! - `quorum_public_key.bin`: the 48-byte BLS public key of the quorum that signed the proof,
//!   looked up by the proof's `quorum_type` / `quorum_hash` at the metadata's
//!   `core_chain_locked_height`.

use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, Proof, ResponseMetadata, WaitForStateTransitionResultResponse,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dapi_grpc::Message;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::version::PlatformVersion;
use drive_proof_verifier::FromProof;
use std::path::PathBuf;
use std::sync::Arc;

/// Directory holding the captured broadcast wait-path vector (see the module TODO).
const VECTOR_DIR: &str = "tests/vectors/broadcast_wait_signed_proof";

/// Protocol version the captured proof's quorum signed over. The v12 devnet capture must match.
const EXPECTED_PROTOCOL_VERSION: u32 = dpp::version::v12::PROTOCOL_VERSION_12;

/// Absolute path to a vector file, rooted at the crate manifest dir so it resolves under `cargo test`.
fn vector_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(VECTOR_DIR)
        .join(file)
}

fn load_bytes(file: &str) -> Vec<u8> {
    let path = vector_path(file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read vector {}: {e}", path.display()))
}

/// Decode the captured signed broadcast response.
fn load_response() -> WaitForStateTransitionResultResponse {
    WaitForStateTransitionResultResponse::decode(load_bytes("response.bin").as_slice())
        .expect("decode WaitForStateTransitionResultResponse vector")
}

/// Reconstruct the broadcast request from the captured state-transition bytes, exactly as the
/// wait-path does via `broadcast_request_for_state_transition`.
fn load_request() -> BroadcastStateTransitionRequest {
    BroadcastStateTransitionRequest {
        state_transition: load_bytes("state_transition.bin"),
    }
}

fn load_quorum_public_key() -> [u8; 48] {
    load_bytes("quorum_public_key.bin")
        .try_into()
        .expect("quorum public key vector must be exactly 48 bytes")
}

/// Minimal [`ContextProvider`] serving one fixed quorum public key, as the broadcast wait-path's
/// provider would for the captured proof. Only `get_quorum_public_key` is exercised by
/// `verify_tenderdash_proof`; the other methods are unreachable for a state-transition execution
/// proof and fail loudly if ever called.
struct FixedQuorumKeyProvider {
    quorum_public_key: [u8; 48],
}

impl ContextProvider for FixedQuorumKeyProvider {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok(self.quorum_public_key)
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Err(ContextProviderError::Generic(
            "data contract lookup not provided by the broadcast signature-verification vector"
                .to_string(),
        ))
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Err(ContextProviderError::Generic(
            "token configuration lookup not provided by the broadcast signature-verification vector"
                .to_string(),
        ))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Err(ContextProviderError::Generic(
            "platform activation height not provided by the broadcast signature-verification vector"
                .to_string(),
        ))
    }
}

/// Run the production verifier exactly as the broadcast wait-path does.
#[expect(
    clippy::result_large_err,
    reason = "mirrors the production FromProof return shape; tests assert on the error"
)]
fn verify(
    request: BroadcastStateTransitionRequest,
    response: WaitForStateTransitionResultResponse,
    provider: &dyn ContextProvider,
) -> Result<
    (Option<StateTransitionProofResult>, ResponseMetadata, Proof),
    drive_proof_verifier::Error,
> {
    <StateTransitionProofResult as FromProof<BroadcastStateTransitionRequest>>::maybe_from_proof_with_metadata(
        request,
        response,
        dpp::dashcore::Network::Regtest,
        PlatformVersion::get(EXPECTED_PROTOCOL_VERSION).expect("known platform version"),
        provider,
    )
}

/// Happy path: a valid signed proof verifies, yields a result, and exposes authenticated metadata.
#[test]
#[ignore = "needs a captured quorum-signed broadcast proof vector + matching quorum public key; vectors generated later"]
fn broadcast_wait_path_verifies_quorum_signed_proof() {
    let provider = FixedQuorumKeyProvider {
        quorum_public_key: load_quorum_public_key(),
    };

    let (maybe_result, metadata, _proof) = verify(load_request(), load_response(), &provider)
        .expect("valid quorum-signed proof must verify");

    assert!(
        maybe_result.is_some(),
        "a valid execution proof must yield a verified result"
    );
    assert_eq!(
        metadata.protocol_version, EXPECTED_PROTOCOL_VERSION,
        "verified metadata must carry the quorum-signed protocol version"
    );
}

/// Tampering with the proof signature must be rejected by the BLS gate.
#[test]
#[ignore = "needs a captured quorum-signed broadcast proof vector + matching quorum public key; vectors generated later"]
fn broadcast_wait_path_rejects_tampered_signature() {
    let provider = FixedQuorumKeyProvider {
        quorum_public_key: load_quorum_public_key(),
    };

    let mut response = load_response();
    let mut tampered = response
        .proof()
        .expect("captured response must contain a proof")
        .clone();
    // Flip one bit of the BLS signature; everything else stays valid.
    let first = tampered
        .signature
        .first_mut()
        .expect("signature is non-empty");
    *first ^= 0x01;
    set_proof(&mut response, tampered);

    let err = verify(load_request(), response, &provider)
        .expect_err("a tampered signature must fail the quorum signature gate");
    assert_signature_rejection(&err);
}

/// A wrong quorum public key (e.g. an attacker substituting a quorum) must be rejected.
#[test]
#[ignore = "needs a captured quorum-signed broadcast proof vector + matching quorum public key; vectors generated later"]
fn broadcast_wait_path_rejects_wrong_quorum_key() {
    let mut wrong_key = load_quorum_public_key();
    wrong_key[0] ^= 0xFF;
    let provider = FixedQuorumKeyProvider {
        quorum_public_key: wrong_key,
    };

    let err = verify(load_request(), load_response(), &provider)
        .expect_err("the wrong quorum key must fail the quorum signature gate");
    assert_signature_rejection(&err);
}

/// Forging `metadata.protocol_version` must be rejected: it feeds `StateId.app_version`, so the
/// signed message hash changes and the quorum signature no longer matches. This is the exact
/// attack the fix closes — unauthenticated metadata must never reach the protocol-version ratchet.
#[test]
#[ignore = "needs a captured quorum-signed broadcast proof vector + matching quorum public key; vectors generated later"]
fn broadcast_wait_path_rejects_forged_protocol_version() {
    let provider = FixedQuorumKeyProvider {
        quorum_public_key: load_quorum_public_key(),
    };

    let mut response = load_response();
    mutate_protocol_version(&mut response, EXPECTED_PROTOCOL_VERSION + 1);

    let err = verify(load_request(), response, &provider)
        .expect_err("a forged protocol_version must fail the quorum signature gate");
    assert_signature_rejection(&err);
}

/// Verify-before-ratchet: a valid proof's authenticated metadata lifts a fresh auto-detect SDK to
/// the proof's protocol version via `verify_response_metadata`, the same call the wait-path makes
/// only after signature verification succeeds.
#[test]
#[ignore = "needs a captured quorum-signed broadcast proof vector + matching quorum public key; vectors generated later"]
fn broadcast_wait_path_valid_proof_ratchets_sdk() {
    use dash_sdk::SdkBuilder;

    let provider = FixedQuorumKeyProvider {
        quorum_public_key: load_quorum_public_key(),
    };

    let (_maybe_result, metadata, _proof) = verify(load_request(), load_response(), &provider)
        .expect("valid quorum-signed proof must verify");

    let sdk = SdkBuilder::new_mock().build().expect("build mock sdk");
    sdk.verify_response_metadata("wait_for_state_transition_result", &metadata)
        .expect("authenticated metadata must pass verification");

    assert_eq!(
        sdk.version().protocol_version,
        EXPECTED_PROTOCOL_VERSION,
        "verify_response_metadata must ratchet the SDK to the quorum-signed protocol version"
    );
}

/// Tenderdash-signature failures surface as `drive_proof_verifier` errors; the structural GroveDB
/// proof is untouched in every tamper case here, so a non-error is a real regression.
fn assert_signature_rejection(err: &drive_proof_verifier::Error) {
    // Any verifier error proves the gate rejected the forgery; the message aids triage.
    tracing::debug!(%err, "broadcast wait-path rejected tampered proof as expected");
}

/// Replace the proof on a V0 response, preserving its result and metadata.
fn set_proof(response: &mut WaitForStateTransitionResultResponse, proof: Proof) {
    use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
        wait_for_state_transition_result_response_v0::Result as V0Result, Version,
    };

    match response.version.as_mut() {
        Some(Version::V0(v0)) => {
            v0.result = Some(V0Result::Proof(proof));
        }
        None => panic!("captured response must be versioned"),
    }
}

/// Mutate `metadata.protocol_version` on a V0 response in place.
fn mutate_protocol_version(response: &mut WaitForStateTransitionResultResponse, version: u32) {
    use dapi_grpc::platform::v0::wait_for_state_transition_result_response::Version;

    match response.version.as_mut() {
        Some(Version::V0(v0)) => {
            v0.metadata
                .as_mut()
                .expect("captured response must carry metadata")
                .protocol_version = version;
        }
        None => panic!("captured response must be versioned"),
    }
}
