//! Tests for identity verification functions

use dpp::version::PlatformVersion;
use js_sys::Uint8Array;
use wasm_bindgen_test::*;
use wasm_drive_verify::identity_verification::*;

mod common;
use common::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_verify_identity_invalid_proof_length() {
    let proof = Uint8Array::from(&mock_proof(10)[..]);
    let identity_id = Uint8Array::from(&mock_identifier()[..]);
    let platform_version = test_platform_version();

    let result = verify_full_identity_by_identity_id(&proof, false, &identity_id, platform_version);
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_verify_identity_invalid_id_length() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_id = Uint8Array::from(&[0u8; 10][..]); // Too short
    let platform_version = test_platform_version();

    let result = verify_full_identity_by_identity_id(&proof, false, &invalid_id, platform_version);
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid identity_id length. Expected 32 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_identity_by_public_key_hash_invalid_length() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_hash = Uint8Array::from(&[0u8; 10][..]); // Too short
    let platform_version = test_platform_version();

    let result =
        verify_full_identity_by_unique_public_key_hash(&proof, &invalid_hash, platform_version);
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid public_key_hash length. Expected 20 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_identity_balance_invalid_id() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_id = Uint8Array::from(&[0u8; 31][..]); // One byte short
    let platform_version = test_platform_version();

    let result =
        verify_identity_balance_for_identity_id(&proof, &invalid_id, false, platform_version);
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid identity_id length. Expected 32 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_multiple_identities_empty_array() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let hashes = js_sys::Array::new();
    let platform_version = test_platform_version();

    let result = verify_full_identities_by_public_key_hashes_vec(&proof, &hashes, platform_version);
    // Should succeed with empty results
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_verify_identity_keys_invalid_request_type() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let identity_id = Uint8Array::from(&mock_identifier()[..]);
    let _invalid_request = wasm_bindgen::JsValue::from_str("invalid");
    let platform_version = test_platform_version();

    let result = verify_identity_keys_by_identity_id(
        &proof,
        &identity_id,
        None,  // specific_key_ids
        false, // with_revision
        false, // with_balance
        false, // is_proof_subset
        None,  // limit
        None,  // offset
        platform_version,
    );
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_verify_identity_nonce_invalid_identity_id() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_identity_id = Uint8Array::from(&[0u8; 16][..]); // Too short
    let platform_version = test_platform_version();

    let result = verify_identity_nonce(&proof, &invalid_identity_id, false, platform_version);
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid identity_id length. Expected 32 bytes",
    );
}

/// A bincode-encoded GroveDB proof envelope discriminant with no payload.
/// The latest protocol version requires at least version 1.
fn envelope_only_proof(version: u32) -> Uint8Array {
    let bytes = bincode::encode_to_vec(version, bincode::config::standard().with_big_endian())
        .expect("encode envelope version");
    Uint8Array::from(&bytes[..])
}

#[wasm_bindgen_test]
fn test_verify_identity_rejects_legacy_v0_envelope() {
    let proof = envelope_only_proof(0);
    let identity_id = Uint8Array::from(&mock_identifier()[..]);
    let platform_version = PlatformVersion::latest().protocol_version;

    let result = verify_full_identity_by_identity_id(&proof, false, &identity_id, platform_version);
    assert_error_contains(
        &result.map(|_| ()),
        "unsupported GroveDB proof envelope version 0 in the proof",
    );
}

#[wasm_bindgen_test]
fn test_verify_identity_by_non_unique_public_key_hash_rejects_v0_inner_proof() {
    let inner_proof = envelope_only_proof(0);
    let outer_proof = envelope_only_proof(1);
    let public_key_hash = Uint8Array::from(&[0u8; 20][..]);
    let platform_version = PlatformVersion::latest().protocol_version;

    let result = verify_full_identity_by_non_unique_public_key_hash(
        Some(inner_proof),
        &outer_proof,
        &public_key_hash,
        None,
        platform_version,
    );
    assert_error_contains(
        &result.map(|_| ()),
        "unsupported GroveDB proof envelope version 0 in the proof",
    );
}

#[wasm_bindgen_test]
fn test_verify_identity_by_non_unique_public_key_hash_rejects_v0_outer_proof() {
    let outer_proof = envelope_only_proof(0);
    let public_key_hash = Uint8Array::from(&[0u8; 20][..]);
    let platform_version = PlatformVersion::latest().protocol_version;

    let result = verify_full_identity_by_non_unique_public_key_hash(
        None,
        &outer_proof,
        &public_key_hash,
        None,
        platform_version,
    );
    assert_error_contains(
        &result.map(|_| ()),
        "unsupported GroveDB proof envelope version 0 in the proof",
    );
}

/// A recorded V1 identity-balance proof from the `drive-proof-verifier`
/// regression corpus, verified through the exported WASM entry point so a
/// valid V1 envelope is known to survive the `Uint8Array` copy and the
/// envelope gate.
#[wasm_bindgen_test]
fn test_verify_identity_balance_accepts_recorded_v1_proof() {
    const PROOF_HEX: &str =
        include_str!("../../rs-drive-proof-verifier/tests/vectors/identity-balance/proof.hex");
    const EXPECTED_ROOT_HASH_HEX: &str =
        "dad905d8fddd7a31089ed57521ff006ec5946b5648d48056bce493357675ab72";
    const RECORDED_PLATFORM_VERSION: u32 = 12;

    let proof_bytes = hex::decode(PROOF_HEX.trim()).expect("decode recorded proof");
    assert_eq!(proof_bytes[0], 1, "corpus fixture must be a V1 envelope");
    let proof = Uint8Array::from(&proof_bytes[..]);
    let identity_id = Uint8Array::from(&[0x77u8; 32][..]);

    let result = verify_identity_balance_for_identity_id(
        &proof,
        &identity_id,
        false,
        RECORDED_PLATFORM_VERSION,
    )
    .expect("recorded V1 proof must verify");

    assert_eq!(
        hex::encode(result.root_hash().to_vec()),
        EXPECTED_ROOT_HASH_HEX
    );
    assert_eq!(result.balance(), Some(5_000_000_000));
}
