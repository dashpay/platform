//! Tests for contract verification functions

use js_sys::Uint8Array;
use wasm_bindgen_test::*;
use wasm_drive_verify::contract_verification::verify_contract::verify_contract;
use wasm_drive_verify::contract_verification::verify_contract_history::verify_contract_history;

mod common;
use common::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_verify_contract_invalid_id_length() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_contract_id = Uint8Array::from(&[0u8; 31][..]); // One byte short
    let platform_version = test_platform_version();

    let result = verify_contract(
        &proof,
        None,
        false,
        false,
        &invalid_contract_id,
        platform_version,
    );
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid contract_id length. Expected 32 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_contract_history_invalid_parameters() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let platform_version = test_platform_version();

    // Test with start_at_date of 0
    let result = verify_contract_history(&proof, &contract_id, 0, None, None, platform_version);
    // Should not panic, actual verification will fail due to mock proof
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_verify_contract_history_large_limit() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let platform_version = test_platform_version();

    // Test with very large limit - should be handled gracefully
    let result = verify_contract_history(
        &proof,
        &contract_id,
        0,           // start_at_date
        Some(50000), // large limit within u16 range
        None,        // offset
        platform_version,
    );
    // Should not panic, actual verification will fail due to mock proof
    assert!(result.is_err());
}
