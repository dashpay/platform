//! Fuzz tests for critical verification paths
//!
//! These tests generate random inputs to test robustness of parsing and validation

use js_sys::{Array, Object, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

mod common;
use common::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Generate random bytes of given length
fn random_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 + 13) as u8).collect()
}

/// Generate random proof with variable size
fn fuzz_proof(size: usize) -> Uint8Array {
    Uint8Array::from(&random_bytes(size)[..])
}

/// Generate random identifier that may or may not be valid
fn fuzz_identifier(valid: bool) -> Uint8Array {
    if valid {
        Uint8Array::from(&random_bytes(32)[..])
    } else {
        let size = (random_bytes(1)[0] % 64) as usize;
        Uint8Array::from(&random_bytes(size)[..])
    }
}

/// Generate random array with variable size
fn fuzz_array(size: usize, element_generator: impl Fn(usize) -> JsValue) -> Array {
    let array = Array::new();
    for i in 0..size {
        array.push(&element_generator(i));
    }
    array
}

#[wasm_bindgen_test]
fn fuzz_identity_verification_with_random_inputs() {
    use wasm_drive_verify::identity_verification::verify_full_identity_by_identity_id;

    // Test with various proof sizes
    for proof_size in [0, 1, 10, 100, 1000, 10000] {
        let proof = fuzz_proof(proof_size);

        // Test with valid and invalid identity IDs
        for valid_id in [true, false] {
            let identity_id = fuzz_identifier(valid_id);
            let result = verify_full_identity_by_identity_id(&proof, false, &identity_id, 1);

            // Should handle gracefully without panic
            match result {
                Ok(_) => assert!(valid_id && proof_size > 0),
                Err(_) => assert!(!valid_id || proof_size == 0),
            }
        }
    }
}

#[wasm_bindgen_test]
fn fuzz_document_query_with_nested_structures() {
    use wasm_drive_verify::document_verification::verify_document_proof;

    let proof = fuzz_proof(1000);
    let contract_id = fuzz_identifier(true);

    // Test with deeply nested query objects
    for depth in [1, 5, 10, 50] {
        let mut query = Object::new();

        // Create nested where clauses
        let where_array = Array::new();
        for i in 0..depth {
            let clause = Array::new();
            clause.push(&JsValue::from_str(&format!("field_{}", i)));
            clause.push(&JsValue::from_str("=="));
            clause.push(&JsValue::from_f64(i as f64));
            where_array.push(&clause);
        }
        js_sys::Reflect::set(&query, &JsValue::from_str("where"), &where_array).unwrap();

        // Create a mock contract JS value (as CBOR bytes)
        let contract_js = JsValue::from(contract_id.clone());
        let where_clauses = JsValue::from(&query);
        let order_by = JsValue::NULL;

        // Should handle without panic (may error due to bounds)
        let _ = verify_document_proof(
            &proof,
            &contract_js,
            "test_doc",
            &where_clauses,
            &order_by,
            None,
            None,
            None,
            false,
            None,
            1,
        );
    }
}

#[wasm_bindgen_test]
fn fuzz_array_inputs_with_mixed_valid_invalid() {
    use wasm_drive_verify::identity_verification::verify_full_identities_by_public_key_hashes_vec;

    let proof = fuzz_proof(1000);

    // Test with arrays containing mix of valid and invalid elements
    for array_size in [0, 1, 10, 100, 1000] {
        let hashes = fuzz_array(array_size, |i| {
            // Alternate between valid and invalid hashes
            if i % 2 == 0 {
                fuzz_identifier(true).into()
            } else {
                fuzz_identifier(false).into()
            }
        });

        // Should handle gracefully
        let _ = verify_full_identities_by_public_key_hashes_vec(&proof, &hashes, 1);
    }
}

#[wasm_bindgen_test]
fn fuzz_platform_version_boundaries() {
    use wasm_drive_verify::identity_verification::verify_full_identity_by_identity_id;

    let proof = fuzz_proof(100);
    let identity_id = fuzz_identifier(true);

    // Test with various platform versions
    for version in [0, 1, 100, 1000, u32::MAX / 2, u32::MAX - 1, u32::MAX] {
        let result = verify_full_identity_by_identity_id(&proof, false, &identity_id, version);
        // Should handle without panic (may error on invalid versions)
        let _ = result;
    }
}

#[wasm_bindgen_test]
fn fuzz_malformed_cbor_inputs() {
    use wasm_drive_verify::contract_verification::verify_contract::verify_contract;

    // Generate malformed CBOR-like data
    for i in 0..100 {
        let mut bytes = random_bytes(100 + i);
        // Add CBOR-like headers
        bytes[0] = 0x80 + (i % 32) as u8;

        let proof = Uint8Array::from(&bytes[..]);
        let contract_id = fuzz_identifier(true);

        // Should handle malformed data gracefully
        let _ = verify_contract(&proof, None, false, false, &contract_id, 1);
    }
}

#[wasm_bindgen_test]
fn fuzz_unicode_and_special_characters() {
    use wasm_drive_verify::document_verification::verify_document_proof;

    let proof = fuzz_proof(100);
    let contract_id = fuzz_identifier(true);

    // Test with various Unicode and special characters
    let special_strings = vec![
        "",
        " ",
        "\n\r\t",
        "🚀🎉🔥",
        "null",
        "undefined",
        "\\x00\\x01\\x02",
        "<script>alert('xss')</script>",
        "'; DROP TABLE users; --",
        std::str::from_utf8(&[0xFF, 0xFE, 0xFD]).unwrap_or("invalid"),
    ];

    for doc_type in special_strings {
        let query = Object::new();
        // Create a mock contract JS value (as CBOR bytes)
        let contract_js = JsValue::from(contract_id.clone());
        let where_clauses = JsValue::from(&query);
        let order_by = JsValue::NULL;

        // Should handle special characters without panic
        let _ = verify_document_proof(
            &proof,
            &contract_js,
            doc_type,
            &where_clauses,
            &order_by,
            None,
            None,
            None,
            false,
            None,
            1,
        );
    }
}
