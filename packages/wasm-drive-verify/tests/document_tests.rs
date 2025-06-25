//! Tests for document verification functions

use js_sys::{Object, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_drive_verify::document_verification::*;

mod common;
use common::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_verify_proof_invalid_contract_id() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_contract_id = Uint8Array::from(&[0u8; 20][..]); // Too short
    let document_type = "test_doc";
    let query = Object::new();
    let platform_version = test_platform_version();

    let result = verify_proof(
        &proof,
        &invalid_contract_id,
        document_type,
        &query,
        platform_version,
    );
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid contract_id length. Expected 32 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_proof_empty_document_type() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let document_type = "";
    let query = Object::new();
    let platform_version = test_platform_version();

    let result = verify_proof(
        &proof,
        &contract_id,
        document_type,
        &query,
        platform_version,
    );
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_verify_single_document_invalid_document_id() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let invalid_document_id = Uint8Array::from(&[0u8; 16][..]); // Too short
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let document_type = "test_doc";
    let platform_version = test_platform_version();

    let result = verify_single_document(
        &proof,
        &invalid_document_id,
        &contract_id,
        document_type,
        platform_version,
    );
    assert_error_contains(
        &result.map(|_| ()),
        "Invalid document_id length. Expected 32 bytes",
    );
}

#[wasm_bindgen_test]
fn test_verify_start_at_document_bounds_check() {
    let proof = Uint8Array::from(&mock_proof(100)[..]);
    let contract_id = Uint8Array::from(&mock_identifier()[..]);
    let document_type = "test_doc";

    // Create a query with nested arrays that might overflow
    let query = Object::new();
    let where_array = js_sys::Array::new();
    for _ in 0..1000 {
        let clause = js_sys::Array::new();
        clause.push(&JsValue::from_str("field"));
        clause.push(&JsValue::from_str("=="));
        clause.push(&JsValue::from_f64(1.0));
        where_array.push(&clause);
    }
    js_sys::Reflect::set(&query, &JsValue::from_str("where"), &where_array).unwrap();

    let platform_version = test_platform_version();

    // Should handle large nested structures gracefully
    let result = verify_start_at_document_in_proof(
        &proof,
        &contract_id,
        document_type,
        &query,
        platform_version,
    );
    // The actual Drive verification will fail, but parsing should not panic
    assert!(result.is_err());
}
