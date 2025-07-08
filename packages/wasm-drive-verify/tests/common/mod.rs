//! Common test utilities for wasm-drive-verify tests

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Generate a mock proof for testing
pub fn mock_proof(size: usize) -> Vec<u8> {
    vec![0xAB; size]
}

/// Generate a mock 32-byte identifier
pub fn mock_identifier() -> [u8; 32] {
    [0xFF; 32]
}

/// Generate a mock 20-byte hash
pub fn mock_hash_160() -> [u8; 20] {
    [0xEE; 20]
}

/// Generate test platform version
pub fn test_platform_version() -> u32 {
    1
}

/// Assert that a result contains an error with a specific message
pub fn assert_error_contains(result: &Result<(), wasm_bindgen::JsValue>, expected: &str) {
    match result {
        Err(js_value) => {
            let error_str = format!("{:?}", js_value);
            assert!(
                error_str.contains(expected),
                "Expected error to contain '{}', but got: {}",
                expected,
                error_str
            );
        }
        Ok(_) => panic!("Expected error containing '{}', but got Ok", expected),
    }
}
