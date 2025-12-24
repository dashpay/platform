//! Tests for identity verification functions

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
