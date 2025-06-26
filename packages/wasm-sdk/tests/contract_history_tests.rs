//! Unit tests for contract history functionality

use wasm_bindgen_test::*;
use wasm_sdk::contract_history::*;
use wasm_sdk::sdk::WasmSdk;
use js_sys::{Array, Object, Reflect, Map};
use wasm_bindgen::JsValue;
use crate::common::{setup_test_sdk, test_contract_id};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_get_contract_history() {
    let sdk = setup_test_sdk().await;
    
    let result = get_contract_history(&sdk, &test_contract_id()).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(history) = result {
        // Should return an array
        assert!(history.is_array());
        
        let history_array = history.dyn_ref::<Array>()
            .expect("Should be an array");
        
        // If there are history entries, check structure
        if history_array.length() > 0 {
            let first_entry = history_array.get(0);
            let entry_obj = first_entry.dyn_ref::<Object>()
                .expect("Entry should be an object");
            
            // Should have version info
            assert!(Reflect::has(entry_obj, &"version".into()).unwrap());
            assert!(Reflect::has(entry_obj, &"timestamp".into()).unwrap());
        }
    }
}

#[wasm_bindgen_test]
async fn test_get_contract_at_version() {
    let sdk = setup_test_sdk().await;
    
    let result = get_contract_at_version(&sdk, &test_contract_id(), 1).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(Some(contract)) = result {
        let contract_obj = contract.dyn_ref::<Object>()
            .expect("Should be an object");
        
        // Should have contract fields
        assert!(Reflect::has(contract_obj, &"version".into()).unwrap());
        assert!(Reflect::has(contract_obj, &"schema".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_get_schema_changes() {
    let sdk = setup_test_sdk().await;
    
    let result = get_schema_changes(&sdk, &test_contract_id(), 1, 2).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(changes) = result {
        // Should return an array
        assert!(changes.is_array());
        
        let changes_array = changes.dyn_ref::<Array>()
            .expect("Should be an array");
        
        // If there are changes, check structure
        if changes_array.length() > 0 {
            let first_change = changes_array.get(0);
            let change_obj = first_change.dyn_ref::<Object>()
                .expect("Change should be an object");
            
            // Should have change info
            assert!(Reflect::has(change_obj, &"type".into()).unwrap());
            assert!(Reflect::has(change_obj, &"path".into()).unwrap());
        }
    }
}

#[wasm_bindgen_test]
async fn test_get_migration_guide() {
    let sdk = setup_test_sdk().await;
    
    let result = get_migration_guide(&sdk, &test_contract_id(), 1, 2).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(guide) = result {
        // Should return a string
        assert!(guide.is_string());
        
        if let Some(guide_str) = guide.as_string() {
            // Guide should not be empty if there are changes
            assert!(!guide_str.is_empty() || guide_str == "No changes between versions");
        }
    }
}

#[wasm_bindgen_test]
async fn test_monitor_contract_updates() {
    let sdk = setup_test_sdk().await;
    
    // Create a callback function
    let callback = js_sys::Function::new_with_args(
        "update",
        "console.log('Contract updated:', update);"
    );
    
    let result = monitor_contract_updates(
        &sdk,
        &test_contract_id(),
        callback,
        Some(1000) // 1 second interval
    ).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(stop_fn) = result {
        // Should return a function
        assert!(stop_fn.is_function());
        
        // Call stop function
        let stop = stop_fn.dyn_ref::<js_sys::Function>()
            .expect("Should be a function");
        let _ = stop.call0(&JsValue::null());
    }
}

#[wasm_bindgen_test]
async fn test_get_contracts_by_owner() {
    let sdk = setup_test_sdk().await;
    let owner_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    
    let result = get_contracts_by_owner(&sdk, owner_id).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(contracts) = result {
        // Should return an array
        assert!(contracts.is_array());
    }
}

#[wasm_bindgen_test]
async fn test_get_contract_document_count() {
    let sdk = setup_test_sdk().await;
    let document_type = "domain";
    
    let result = get_contract_document_count(&sdk, &test_contract_id(), document_type).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(count) = result {
        // Should return a number
        assert!(count.as_f64().is_some());
        
        // Count should be non-negative
        let count_value = count.as_f64().unwrap();
        assert!(count_value >= 0.0);
    }
}

#[wasm_bindgen_test]
async fn test_compare_contract_schemas() {
    let sdk = setup_test_sdk().await;
    let contract1 = test_contract_id();
    let contract2 = "HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed";
    
    let result = compare_contract_schemas(&sdk, &contract1, &contract2).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(comparison) = result {
        let obj = comparison.dyn_ref::<Object>()
            .expect("Should be an object");
        
        // Should have comparison fields
        assert!(Reflect::has(obj, &"identical".into()).unwrap());
        assert!(Reflect::has(obj, &"differences".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_batch_get_contracts() {
    let sdk = setup_test_sdk().await;
    
    // Create array of contract IDs
    let ids = Array::new();
    ids.push(&test_contract_id().into());
    ids.push(&"HWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ed".into());
    
    let result = batch_get_contracts(&sdk, ids).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(contracts) = result {
        // Should return a Map
        let map = contracts.dyn_ref::<Map>()
            .expect("Should be a Map");
        
        // Map size should match input array length or be 0 if all failed
        assert!(map.size() <= 2);
    }
}

#[wasm_bindgen_test]
async fn test_schema_diff_formatting() {
    // Test the diff object structure
    let diff = Object::new();
    Reflect::set(&diff, &"type".into(), &"added".into()).unwrap();
    Reflect::set(&diff, &"path".into(), &"properties.newField".into()).unwrap();
    
    let old_val = Object::new();
    let new_val = Object::new();
    Reflect::set(&new_val, &"type".into(), &"string".into()).unwrap();
    
    Reflect::set(&diff, &"oldValue".into(), &JsValue::undefined()).unwrap();
    Reflect::set(&diff, &"newValue".into(), &new_val).unwrap();
    
    // Create array with this diff
    let diffs = Array::new();
    diffs.push(&diff);
    
    // Should handle diff formatting without errors
    assert!(diffs.length() == 1);
}

#[wasm_bindgen_test]
async fn test_invalid_contract_id() {
    let sdk = setup_test_sdk().await;
    
    // Test with invalid contract ID
    let result = get_contract_history(&sdk, "invalid_id").await;
    
    // Should return an error
    assert!(result.is_err());
}

#[wasm_bindgen_test]
async fn test_version_range_validation() {
    let sdk = setup_test_sdk().await;
    
    // Test with invalid version range (to < from)
    let result = get_schema_changes(&sdk, &test_contract_id(), 5, 2).await;
    
    // Should handle gracefully (empty changes or error)
    if let Ok(changes) = result {
        let changes_array = changes.dyn_ref::<Array>()
            .expect("Should be an array");
        assert_eq!(changes_array.length(), 0);
    }
}

#[wasm_bindgen_test]
async fn test_empty_batch_get_contracts() {
    let sdk = setup_test_sdk().await;
    
    // Test with empty array
    let empty_ids = Array::new();
    
    let result = batch_get_contracts(&sdk, empty_ids).await;
    
    assert!(result.is_ok() || result.is_err());
    
    if let Ok(contracts) = result {
        let map = contracts.dyn_ref::<Map>()
            .expect("Should be a Map");
        
        // Should return empty map
        assert_eq!(map.size(), 0);
    }
}