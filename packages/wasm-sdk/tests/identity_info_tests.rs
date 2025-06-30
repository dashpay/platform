//! Unit tests for identity info functionality

use crate::common::{setup_test_sdk, test_identity_id};
use js_sys::{Array, Map, Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use wasm_sdk::identity_info::*;
use wasm_sdk::sdk::WasmSdk;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_get_identity_info() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_info(&sdk, &test_identity_id()).await;

    // Should return a result
    assert!(result.is_ok() || result.is_err());

    if let Ok(info) = result {
        let obj = info.dyn_ref::<Object>().expect("Should be an object");

        // Should have expected fields
        assert!(Reflect::has(obj, &"id".into()).unwrap());
        assert!(Reflect::has(obj, &"balance".into()).unwrap());
        assert!(Reflect::has(obj, &"revision".into()).unwrap());
        assert!(Reflect::has(obj, &"publicKeys".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_balance() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_balance(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(balance) = result {
        // Should return a number
        assert!(balance.as_f64().is_some());

        // Balance should be non-negative
        let balance_value = balance.as_f64().unwrap();
        assert!(balance_value >= 0.0);
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_revision() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_revision(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(revision) = result {
        // Should return a number
        assert!(revision.as_f64().is_some());

        // Revision should be non-negative
        let revision_value = revision.as_f64().unwrap();
        assert!(revision_value >= 0.0);
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_public_keys() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_public_keys(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(keys) = result {
        // Should return an array
        assert!(keys.is_array());

        let keys_array = keys.dyn_ref::<Array>().expect("Should be an array");

        // If there are keys, check structure
        if keys_array.length() > 0 {
            let first_key = keys_array.get(0);
            let key_obj = first_key
                .dyn_ref::<Object>()
                .expect("Key should be an object");

            // Should have key properties
            assert!(Reflect::has(key_obj, &"id".into()).unwrap());
            assert!(Reflect::has(key_obj, &"type".into()).unwrap());
            assert!(Reflect::has(key_obj, &"purpose".into()).unwrap());
        }
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_key_by_id() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_key_by_id(&sdk, &test_identity_id(), 0).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(Some(key)) = result {
        let key_obj = key.dyn_ref::<Object>().expect("Key should be an object");

        // Should have key properties
        assert!(Reflect::has(key_obj, &"id".into()).unwrap());
        assert!(Reflect::has(key_obj, &"type".into()).unwrap());
        assert!(Reflect::has(key_obj, &"data".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_credit_withdrawal_info() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_credit_withdrawal_info(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(info) = result {
        let obj = info.dyn_ref::<Object>().expect("Should be an object");

        // Should have withdrawal info fields
        assert!(Reflect::has(obj, &"withdrawalAddress".into()).unwrap());
        assert!(Reflect::has(obj, &"coreFeePerByte".into()).unwrap());
        assert!(Reflect::has(obj, &"minWithdrawal".into()).unwrap());
        assert!(Reflect::has(obj, &"maxWithdrawal".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_check_identity_exists() {
    let sdk = setup_test_sdk().await;

    let result = check_identity_exists(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(exists) = result {
        // Should return a boolean
        assert!(exists.is_boolean());
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_metadata() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_metadata(&sdk, &test_identity_id()).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(metadata) = result {
        // Should return a Map
        let map = metadata.dyn_ref::<Map>().expect("Should be a Map");

        // Check if it has any entries
        assert!(map.size() >= 0);
    }
}

#[wasm_bindgen_test]
async fn test_get_identity_contract_bounds() {
    let sdk = setup_test_sdk().await;
    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";

    let result = get_identity_contract_bounds(&sdk, &test_identity_id(), contract_id).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(bounds) = result {
        let obj = bounds.dyn_ref::<Object>().expect("Should be an object");

        // Should have bounds info
        assert!(Reflect::has(obj, &"documentsCreated".into()).unwrap());
        assert!(Reflect::has(obj, &"documentsDeleted".into()).unwrap());
        assert!(Reflect::has(obj, &"storageUsed".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_monitor_identity_balance() {
    let sdk = setup_test_sdk().await;

    // Create a callback function
    let callback =
        js_sys::Function::new_with_args("balance", "console.log('Balance updated:', balance);");

    let result = monitor_identity_balance(
        &sdk,
        &test_identity_id(),
        callback,
        Some(1000), // 1 second interval
    )
    .await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(stop_fn) = result {
        // Should return a function
        assert!(stop_fn.is_function());

        // Call stop function
        let stop = stop_fn
            .dyn_ref::<js_sys::Function>()
            .expect("Should be a function");
        let _ = stop.call0(&JsValue::null());
    }
}

#[wasm_bindgen_test]
async fn test_batch_get_identities() {
    let sdk = setup_test_sdk().await;

    // Create array of identity IDs
    let ids = Array::new();
    ids.push(&test_identity_id().into());
    ids.push(&"HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed".into());

    let result = batch_get_identities(&sdk, ids).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(identities) = result {
        // Should return a Map
        let map = identities.dyn_ref::<Map>().expect("Should be a Map");

        // Map size should match input array length or be 0 if all failed
        assert!(map.size() <= 2);
    }
}

#[wasm_bindgen_test]
async fn test_empty_batch_get_identities() {
    let sdk = setup_test_sdk().await;

    // Test with empty array
    let empty_ids = Array::new();

    let result = batch_get_identities(&sdk, empty_ids).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(identities) = result {
        let map = identities.dyn_ref::<Map>().expect("Should be a Map");

        // Should return empty map
        assert_eq!(map.size(), 0);
    }
}

#[wasm_bindgen_test]
async fn test_invalid_identity_id() {
    let sdk = setup_test_sdk().await;

    // Test with invalid identity ID
    let result = get_identity_info(&sdk, "invalid_id").await;

    // Should return an error
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_identity_key_purposes() {
    let sdk = setup_test_sdk().await;

    let result = get_identity_public_keys(&sdk, &test_identity_id()).await;

    if let Ok(keys) = result {
        let keys_array = keys.dyn_ref::<Array>().expect("Should be an array");

        // Check key purposes if there are keys
        if keys_array.length() > 0 {
            for i in 0..keys_array.length() {
                let key = keys_array.get(i);
                let key_obj = key.dyn_ref::<Object>().expect("Key should be an object");

                let purpose =
                    Reflect::get(key_obj, &"purpose".into()).expect("Should have purpose");

                // Purpose should be a valid number (0-5)
                if let Some(purpose_num) = purpose.as_f64() {
                    assert!(purpose_num >= 0.0 && purpose_num <= 5.0);
                }
            }
        }
    }
}
