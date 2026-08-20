//! Tenderdash quorum-signature regression tests, seeded from the Dash Core
//! `quorum_sig_vectors.json` fixture (tenderdash v1.5.1, basic BLS scheme).
//! The positive case runs a full identity-balance verification whose
//! envelope carries a real quorum signature over the state; the negatives
//! flip exactly one ingredient each (signature bit, quorum key, block id
//! hash, or a `ResponseMetadata` field folded into the signed `StateId`)
//! and must fail with an invalid-signature error -- proving the signature
//! actually binds those inputs.

#![cfg(feature = "mocks")]

mod common;

use common::{identifier, load_case, Case, ErrorClass, Expected, RequestSpec, NETWORK};
use dapi_grpc::platform::v0::{
    self as platform, get_identity_balance_request, get_identity_balance_response,
};
use drive_proof_verifier::types::IdentityBalance;
use drive_proof_verifier::{Error, FromProof};

// The crate's own Error type is what the public API returns; its size is
// not this test's concern.
#[allow(clippy::result_large_err)]
fn verify_balance(case: &Case) -> Result<Option<IdentityBalance>, Error> {
    verify_balance_with_metadata(case, case.metadata())
}

/// Like [`verify_balance`], but lets the caller substitute the response
/// metadata that gets carried alongside the case's untouched proof envelope
/// -- used to prove the metadata itself is inside the signed `StateId`.
#[allow(clippy::result_large_err)]
fn verify_balance_with_metadata(
    case: &Case,
    metadata: platform::ResponseMetadata,
) -> Result<Option<IdentityBalance>, Error> {
    let RequestSpec::IdentityBalance { identity_id } = &case.manifest.request else {
        panic!("{}: expected an identity_balance request", case.name);
    };
    let request = platform::GetIdentityBalanceRequest {
        version: Some(get_identity_balance_request::Version::V0(
            get_identity_balance_request::GetIdentityBalanceRequestV0 {
                id: identifier(identity_id).to_vec(),
                prove: true,
            },
        )),
    };
    let response = platform::GetIdentityBalanceResponse {
        version: Some(get_identity_balance_response::Version::V0(
            get_identity_balance_response::GetIdentityBalanceResponseV0 {
                metadata: Some(metadata),
                result: Some(
                    get_identity_balance_response::get_identity_balance_response_v0::Result::Proof(
                        case.grpc_proof(),
                    ),
                ),
            },
        )),
    };
    IdentityBalance::maybe_from_proof(
        request,
        response,
        NETWORK,
        case.platform_version(),
        &case.provider(),
    )
}

fn expect_invalid_signature(name: &str) {
    let case = load_case(name);
    let Expected::Error { class } = case.manifest.expected else {
        panic!("{name}: manifest expectation mismatch");
    };
    assert_eq!(class, ErrorClass::InvalidSignature);
    let error = verify_balance(&case).expect_err("quorum signature check must fail");
    // A wrong key or wrong signed payload fails the pairing check
    // (InvalidSignature); a bit-flipped signature may already fail G2 point
    // decompression (SignatureVerificationError). Both are proper
    // rejections of a bad quorum signature.
    assert!(
        matches!(
            error,
            Error::InvalidSignature { .. } | Error::SignatureVerificationError { .. }
        ),
        "{name}: expected a signature rejection, got: {error:?}"
    );
}

#[test]
fn valid_quorum_signature_verifies() {
    let case = load_case("quorum-sig-valid");
    let Expected::IdentityBalance { balance: expected } = case.manifest.expected else {
        panic!("manifest expectation mismatch");
    };
    let balance = verify_balance(&case)
        .expect("envelope with a valid quorum signature must verify end to end");
    assert_eq!(balance, Some(expected));
}

#[test]
fn tampered_signature_is_rejected() {
    expect_invalid_signature("quorum-sig-tampered-signature");
}

#[test]
fn wrong_quorum_key_is_rejected() {
    expect_invalid_signature("quorum-sig-wrong-quorum-key");
}

#[test]
fn wrong_block_id_hash_is_rejected() {
    expect_invalid_signature("quorum-sig-wrong-block-id-hash");
}

// The quorum signs a `StateId` built from `height`, `core_chain_locked_height`,
// `time_ms`, and the app hash (see `verify_tenderdash_proof` in `src/verify.rs`).
// The proof envelope and signature bytes are untouched here -- only the
// out-of-band `ResponseMetadata` a server could independently lie about is
// mutated -- so a pass here would mean the metadata is never actually bound
// by the signature.
fn expect_metadata_tamper_breaks_signature(mutate: impl Fn(&mut platform::ResponseMetadata)) {
    let case = load_case("quorum-sig-valid");
    let mut metadata = case.metadata();
    mutate(&mut metadata);
    let error = verify_balance_with_metadata(&case, metadata)
        .expect_err("tampered metadata must break the quorum signature check");
    assert!(
        matches!(
            error,
            Error::InvalidSignature { .. } | Error::SignatureVerificationError { .. }
        ),
        "expected a signature rejection, got: {error:?}"
    );
}

#[test]
fn tampered_height_breaks_signature() {
    expect_metadata_tamper_breaks_signature(|metadata| metadata.height += 1);
}

#[test]
fn tampered_time_ms_breaks_signature() {
    expect_metadata_tamper_breaks_signature(|metadata| metadata.time_ms += 1);
}

#[test]
fn tampered_core_chain_locked_height_breaks_signature() {
    expect_metadata_tamper_breaks_signature(|metadata| metadata.core_chain_locked_height += 1);
}
