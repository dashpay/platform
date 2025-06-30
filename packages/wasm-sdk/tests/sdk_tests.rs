//! SDK initialization and basic functionality tests

use wasm_bindgen_test::*;
use wasm_sdk::{context_provider::ContextProvider, sdk::WasmSdk, start};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_wasm_initialization() {
    // Test that WASM module can be initialized
    let result = start().await;
    assert!(result.is_ok(), "WASM module should initialize successfully");
}

#[wasm_bindgen_test]
async fn test_sdk_creation() {
    start().await.expect("Failed to start WASM");

    // Test mainnet SDK creation
    let mainnet_sdk = WasmSdk::new("mainnet".to_string(), None);
    assert!(mainnet_sdk.is_ok(), "Should create mainnet SDK");
    assert_eq!(mainnet_sdk.unwrap().network(), "mainnet");

    // Test testnet SDK creation
    let testnet_sdk = WasmSdk::new("testnet".to_string(), None);
    assert!(testnet_sdk.is_ok(), "Should create testnet SDK");
    assert_eq!(testnet_sdk.unwrap().network(), "testnet");

    // Test devnet SDK creation
    let devnet_sdk = WasmSdk::new("devnet".to_string(), None);
    assert!(devnet_sdk.is_ok(), "Should create devnet SDK");
    assert_eq!(devnet_sdk.unwrap().network(), "devnet");
}

#[wasm_bindgen_test]
async fn test_sdk_is_ready() {
    start().await.expect("Failed to start WASM");

    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    assert!(sdk.is_ready(), "SDK should be ready after creation");
}

#[wasm_bindgen_test]
async fn test_invalid_network() {
    start().await.expect("Failed to start WASM");

    let invalid_sdk = WasmSdk::new("invalid_network".to_string(), None);
    assert!(invalid_sdk.is_err(), "Should fail with invalid network");

    // Test empty network string
    let empty_network_sdk = WasmSdk::new("".to_string(), None);
    assert!(
        empty_network_sdk.is_err(),
        "Should fail with empty network string"
    );

    // Test network with spaces
    let space_network_sdk = WasmSdk::new("test net".to_string(), None);
    assert!(
        space_network_sdk.is_err(),
        "Should fail with network containing spaces"
    );

    // Test case sensitivity
    let uppercase_sdk = WasmSdk::new("TESTNET".to_string(), None);
    assert!(
        uppercase_sdk.is_err(),
        "Should fail with uppercase network name"
    );

    // Test network with special characters
    let special_char_sdk = WasmSdk::new("test-net!".to_string(), None);
    assert!(
        special_char_sdk.is_err(),
        "Should fail with special characters in network name"
    );
}

#[wasm_bindgen_test]
async fn test_context_provider() {
    use wasm_bindgen::prelude::*;

    start().await.expect("Failed to start WASM");

    // Create a mock context provider
    #[wasm_bindgen]
    pub struct MockContextProvider;

    #[wasm_bindgen]
    impl MockContextProvider {
        #[wasm_bindgen(js_name = getBlockHeight)]
        pub async fn get_block_height(&self) -> Result<JsValue, JsValue> {
            Ok(JsValue::from(12345))
        }

        #[wasm_bindgen(js_name = getCoreChainLockedHeight)]
        pub async fn get_core_chain_locked_height(&self) -> Result<JsValue, JsValue> {
            Ok(JsValue::from(12340))
        }

        #[wasm_bindgen(js_name = getTimeMillis)]
        pub async fn get_time_millis(&self) -> Result<JsValue, JsValue> {
            Ok(JsValue::from(1234567890))
        }
    }

    // Test SDK with custom context provider
    let provider = MockContextProvider;
    let provider_js = JsValue::from(provider);

    let sdk = WasmSdk::new(
        "testnet".to_string(),
        Some(ContextProvider::from(provider_js)),
    );
    assert!(
        sdk.is_ok(),
        "Should create SDK with custom context provider"
    );
}
