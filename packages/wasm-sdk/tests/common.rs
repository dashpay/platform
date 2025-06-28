//! Common test utilities and setup

use wasm_bindgen_test::*;
use wasm_sdk::{sdk::WasmSdk, start};

wasm_bindgen_test_configure!(run_in_browser);

/// Initialize test environment
pub async fn setup_test_sdk() -> WasmSdk {
    // Initialize WASM module
    start().await.expect("Failed to start WASM module");

    // Create SDK instance for testnet
    WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK")
}

/// Generate test identity ID
pub fn test_identity_id() -> String {
    "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec".to_string()
}

/// Generate test contract ID
pub fn test_contract_id() -> String {
    "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec".to_string()
}

/// Generate test document ID
pub fn test_document_id() -> String {
    "4mZmxva49PBb7BE7srw9o3gixvDfj1dAx8x2dmm8v9Xp".to_string()
}

/// Generate test transaction bytes
pub fn test_transaction_bytes() -> Vec<u8> {
    vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
}

/// Generate test instant lock bytes
pub fn test_instant_lock_bytes() -> Vec<u8> {
    vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
}

/// Generate test private key
pub fn test_private_key() -> Vec<u8> {
    vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ]
}

/// Generate test public key
pub fn test_public_key() -> Vec<u8> {
    vec![
        0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
        0x20, 0x21, 0x22,
    ]
}

/// Assert that a JsValue is not null or undefined
pub fn assert_not_null(value: &wasm_bindgen::JsValue) {
    assert!(!value.is_null(), "Value should not be null");
    assert!(!value.is_undefined(), "Value should not be undefined");
}
