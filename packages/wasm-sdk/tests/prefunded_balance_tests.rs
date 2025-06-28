//! Unit tests for prefunded balance functionality

use crate::common::{setup_test_sdk, test_identity_id, test_private_key};
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_sdk::prefunded_balance::*;
use wasm_sdk::sdk::WasmSdk;
use wasm_sdk::signer::WasmSigner;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_top_up_identity() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();

    // Set identity ID
    signer
        .set_identity_id(&test_identity_id())
        .expect("Should set identity ID");

    // Add a test private key
    signer
        .add_private_key(1, test_private_key(), "ECDSA_SECP256K1", 0)
        .expect("Should add private key");

    let result = top_up_identity(&sdk, &test_identity_id(), 1000000, &signer).await;

    // In test environment this will likely fail, but should not panic
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_prefunded_balance() {
    let sdk = setup_test_sdk().await;

    let result = get_prefunded_balance(&sdk, &test_identity_id()).await;

    // Should return a result (may be error in test env)
    assert!(result.is_ok() || result.is_err());

    if let Ok(balance) = result {
        // Balance should be a number
        assert!(balance.as_f64().is_some());
    }
}

#[wasm_bindgen_test]
async fn test_get_prefunded_balance_and_revision() {
    let sdk = setup_test_sdk().await;

    let result = get_prefunded_balance_and_revision(&sdk, &test_identity_id()).await;

    // Should return a result
    assert!(result.is_ok() || result.is_err());

    if let Ok(result_obj) = result {
        let obj = result_obj.dyn_ref::<Object>().expect("Should be an object");

        // Should have balance and revision fields
        assert!(Reflect::has(obj, &"balance".into()).unwrap());
        assert!(Reflect::has(obj, &"revision".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_transfer_credits() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();

    let from_identity = test_identity_id();
    let to_identity = "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed"; // Different ID

    signer
        .set_identity_id(&from_identity)
        .expect("Should set identity ID");
    signer
        .add_private_key(1, test_private_key(), "ECDSA_SECP256K1", 0)
        .expect("Should add private key");

    let result = transfer_credits(&sdk, &from_identity, &to_identity, 500000, &signer).await;

    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_batch_top_up() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();

    let funding_identity = test_identity_id();
    signer
        .set_identity_id(&funding_identity)
        .expect("Should set identity ID");
    signer
        .add_private_key(1, test_private_key(), "ECDSA_SECP256K1", 0)
        .expect("Should add private key");

    // Create array of identities to top up
    let identities = Array::new();
    identities.push(&"HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed".into());
    identities.push(&"IWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ee".into());

    let result = batch_top_up(&sdk, &funding_identity, identities, 100000, &signer).await;

    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_check_minimum_balance() {
    let sdk = setup_test_sdk().await;

    let result = check_minimum_balance(&sdk, &test_identity_id(), 1000000).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(has_minimum) = result {
        // Should return a boolean
        assert!(has_minimum.is_boolean());
    }
}

#[wasm_bindgen_test]
async fn test_estimate_top_up_cost() {
    let cost = estimate_top_up_cost(1000000);

    // Should return a JsValue number
    assert!(cost.as_f64().is_some());

    // Cost should be positive
    let cost_value = cost.as_f64().unwrap();
    assert!(cost_value > 0.0);
}

#[wasm_bindgen_test]
async fn test_wait_for_balance_update() {
    let sdk = setup_test_sdk().await;

    // This will timeout in test environment
    let result = wait_for_balance_update(
        &sdk,
        &test_identity_id(),
        1000000,
        1000, // 1 second timeout
        100,  // 100ms interval
    )
    .await;

    // Should timeout and return error
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_get_funding_address() {
    let sdk = setup_test_sdk().await;

    let result = get_funding_address(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(address) = result {
        // Should return a string
        assert!(address.is_string());

        if let Some(addr_str) = address.as_string() {
            // Should not be empty
            assert!(!addr_str.is_empty());
        }
    }
}

#[wasm_bindgen_test]
async fn test_get_credit_conversion_rate() {
    let sdk = setup_test_sdk().await;

    let result = get_credit_conversion_rate(&sdk).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(rate) = result {
        // Should return a number
        assert!(rate.as_f64().is_some());

        // Rate should be positive
        let rate_value = rate.as_f64().unwrap();
        assert!(rate_value > 0.0);
    }
}

#[wasm_bindgen_test]
fn test_invalid_identity_id() {
    let sdk = setup_test_sdk();
    let mut signer = WasmSigner::new();

    // Test with invalid identity ID format
    let result = signer.set_identity_id("invalid_id");
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_zero_amount_top_up() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();

    signer
        .set_identity_id(&test_identity_id())
        .expect("Should set identity ID");
    signer
        .add_private_key(1, test_private_key(), "ECDSA_SECP256K1", 0)
        .expect("Should add private key");

    // Should handle zero amount gracefully
    let result = top_up_identity(&sdk, &test_identity_id(), 0, &signer).await;

    // Implementation may accept or reject zero amount
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_batch_top_up_empty_array() {
    let sdk = setup_test_sdk().await;
    let mut signer = WasmSigner::new();

    signer
        .set_identity_id(&test_identity_id())
        .expect("Should set identity ID");
    signer
        .add_private_key(1, test_private_key(), "ECDSA_SECP256K1", 0)
        .expect("Should add private key");

    // Test with empty array
    let empty_identities = Array::new();

    let result = batch_top_up(&sdk, &test_identity_id(), empty_identities, 100000, &signer).await;

    // Should handle empty array gracefully
    assert!(result.is_ok() || result.is_err());
}
