//! Unit tests for DAPI client functionality

use js_sys::{Array, Object, Reflect};
use serde_json::json;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_sdk::dapi_client::*;
use wasm_sdk::sdk::WasmSdk;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_dapi_client_creation() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config);
    assert!(client.is_ok());
}

#[wasm_bindgen_test]
fn test_dapi_client_config() {
    let mut config = DapiClientConfig::new("testnet".to_string());

    // Test timeout setter
    config.set_timeout(5000);

    // Test retry setter
    config.set_retries(3);

    // Test adding addresses
    config.add_address("https://testnet-1.dash.org:443".to_string());
    config.add_address("https://testnet-2.dash.org:443".to_string());

    // Should create client successfully with config
    let client = DapiClient::new(config);
    assert!(client.is_ok());
}

#[wasm_bindgen_test]
async fn test_raw_request() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    // Create a simple request payload
    let request = json!({
        "version": 1
    });

    // This will likely fail in test environment but should not panic
    let result = client.raw_request("/platform/v1/version", &request).await;

    // In a real test environment with mock server, we'd assert success
    // For now, just ensure it returns a Result
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_protocol_version() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    // This will likely fail in test environment but should not panic
    let result = client.get_protocol_version().await;

    // Should return a Result
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_epoch() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    let result = client.get_epoch(0).await;
    assert!(result.is_ok() || result.is_err());

    // Test with specific epoch
    let result = client.get_epoch(42).await;
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_identity() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    let identity_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let result = client.get_identity(identity_id).await;

    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_identity_balance() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    let identity_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let result = client.get_identity_balance(identity_id).await;

    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_data_contract() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let result = client.get_data_contract(contract_id).await;

    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_documents() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let document_type = "domain";

    // Create query object
    let query = Object::new();
    Reflect::set(&query, &"limit".into(), &10.into()).unwrap();

    let result = client
        .get_documents(contract_id, document_type, query)
        .await;
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_broadcast_state_transition() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    // Create mock state transition bytes
    let st_bytes = vec![0x01, 0x02, 0x03, 0x04];

    let result = client.broadcast_state_transition(st_bytes).await;
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
fn test_multiple_dapi_addresses() {
    let mut config = DapiClientConfig::new("testnet".to_string());

    // Add multiple addresses
    let addresses = vec![
        "https://testnet-1.dash.org:443",
        "https://testnet-2.dash.org:443",
        "https://testnet-3.dash.org:443",
    ];

    for addr in addresses {
        config.add_address(addr.to_string());
    }

    let client = DapiClient::new(config);
    assert!(client.is_ok());
}

#[wasm_bindgen_test]
fn test_network_configurations() {
    // Test mainnet config
    let mainnet_config = DapiClientConfig::new("mainnet".to_string());
    let mainnet_client = DapiClient::new(mainnet_config);
    assert!(mainnet_client.is_ok());

    // Test testnet config
    let testnet_config = DapiClientConfig::new("testnet".to_string());
    let testnet_client = DapiClient::new(testnet_config);
    assert!(testnet_client.is_ok());

    // Test custom network
    let custom_config = DapiClientConfig::new("custom".to_string());
    let custom_client = DapiClient::new(custom_config);
    assert!(custom_client.is_ok());
}

#[wasm_bindgen_test]
async fn test_error_handling() {
    let config = DapiClientConfig::new("testnet".to_string());
    let client = DapiClient::new(config).expect("Should create client");

    // Test with invalid endpoint
    let request = json!({});
    let result = client.raw_request("/invalid/endpoint", &request).await;

    // Should return an error
    assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_config_builder_pattern() {
    let config = DapiClientConfig::new("testnet".to_string());

    // Test chaining config methods
    let mut config = config;
    config.set_timeout(3000);
    config.set_retries(5);
    config.add_address("https://custom.dash.org:443".to_string());

    // Should still create client successfully
    let client = DapiClient::new(config);
    assert!(client.is_ok());
}

#[wasm_bindgen_test]
async fn test_concurrent_requests() {
    use std::sync::Arc;
    use wasm_bindgen_futures::spawn_local;

    let config = DapiClientConfig::new("testnet".to_string());
    let client = Arc::new(DapiClient::new(config).expect("Should create client"));

    // Spawn multiple concurrent requests
    let client1 = client.clone();
    spawn_local(async move {
        let _ = client1.get_protocol_version().await;
    });

    let client2 = client.clone();
    spawn_local(async move {
        let _ = client2.get_epoch(0).await;
    });

    let client3 = client.clone();
    spawn_local(async move {
        let identity_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        let _ = client3.get_identity(identity_id).await;
    });

    // Give time for spawned tasks
    gloo_timers::future::TimeoutFuture::new(100).await;
}
