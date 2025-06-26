//! Tests for token verification functions

use js_sys::{Array, Uint8Array};
use wasm_bindgen_test::*;
use wasm_drive_verify::token_verification::verify_token_balance_for_identity_id::verify_token_balance_for_identity_id;
use wasm_drive_verify::token_verification::verify_token_balances_for_identity_ids::verify_token_balances_for_identity_ids_vec;
use wasm_drive_verify::token_verification::verify_token_direct_selling_prices::verify_token_direct_selling_prices_vec;
use wasm_drive_verify::token_verification::verify_token_statuses::verify_token_statuses_vec;

mod common;
use common::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_verify_token_balance_invalid_contract_id() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_contract_id = Uint8Array::from(&[0u8; 20][..]); // Too short
    let identity_id = Uint8Array::from(&mock_identifier()[..]);
    let platform_version = test_platform_version();

    let result = verify_token_balance_for_identity_id(
        &proof,
        &invalid_contract_id,
        &identity_id,
        false, // verify_subset_of_proof
        platform_version,
    );
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid contract_id length. Expected 32 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_token_balances_empty_identity_array() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let identity_ids = Array::new();
    let platform_version = test_platform_version();

    let result = verify_token_balances_for_identity_ids_vec(
        &proof,
        &contract_id,
        false, // verify_subset_of_proof
        &identity_ids,
        platform_version,
    );
    // Should succeed with empty results
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_verify_token_balances_invalid_identity_in_array() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let identity_ids = Array::new();

    // Add valid identity
    identity_ids.push(&Uint8Array::from(&mock_identifier()[..]));
    // Add invalid identity
    identity_ids.push(&Uint8Array::from(&[0u8; 10][..])); // Too short

    let platform_version = test_platform_version();

    let result = verify_token_balances_for_identity_ids_vec(
        &proof,
        &contract_id,
        false, // verify_subset_of_proof
        &identity_ids,
        platform_version,
    );
    assert_error_contains(&result.map(|_| ()), "Invalid identity_id at index 1");
}

#[wasm_bindgen_test]
fn test_verify_token_statuses_empty_array() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_ids = Array::new();
    let platform_version = test_platform_version();

    let result = verify_token_statuses_vec(&proof, &contract_ids, false, platform_version);
    // Should succeed with empty results
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
fn test_verify_token_direct_selling_prices_mixed_valid_invalid() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_ids = Array::new();

    // Add valid contract ID
    contract_ids.push(&Uint8Array::from(&mock_identifier()[..]));
    // Add invalid contract ID
    contract_ids.push(&Uint8Array::from(&[0u8; 30][..])); // Too short

    let platform_version = test_platform_version();

    let result =
        verify_token_direct_selling_prices_vec(&proof, &contract_ids, false, platform_version);
    assert_error_contains(&result.map(|_| ()), "Invalid contract_id at index 1");
}
