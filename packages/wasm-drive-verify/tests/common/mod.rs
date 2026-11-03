//! Common test utilities for wasm-drive-verify tests

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Generate a mock proof for testing.
///
/// The buffer starts with the bincode-encoded GroveDB V1 envelope
/// discriminant so it clears the envelope policy that every public entry
/// point applies first; the rest is filler that never decodes as a proof.
pub fn mock_proof(size: usize) -> Vec<u8> {
    let mut proof = bincode::encode_to_vec(1u32, bincode::config::standard().with_big_endian())
        .expect("encode V1 envelope discriminant");
    proof.resize(size.max(proof.len()), 0xAB);
    proof
}

/// Generate a mock 32-byte identifier
pub fn mock_identifier() -> [u8; 32] {
    [0xFF; 32]
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
