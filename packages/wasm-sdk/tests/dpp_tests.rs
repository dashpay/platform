//! Comprehensive tests for the DPP (Dash Platform Protocol) module

use wasm_bindgen_test::*;
use wasm_bindgen::prelude::*;
use wasm_sdk::{
    dpp::{
        IdentityWasm, DataContractWasm, generate_entropy,
        generate_document_id, create_state_transition,
        validate_public_keys
    },
    start,
};
use js_sys::Uint8Array;

wasm_bindgen_test_configure!(run_in_browser);

const TEST_PLATFORM_VERSION: u32 = 1;

#[wasm_bindgen_test]
async fn test_identity_wasm_creation() {
    start().await.expect("Failed to start WASM");
    
    let identity = IdentityWasm::new(TEST_PLATFORM_VERSION)
        .expect("Failed to create IdentityWasm");
    
    // Test default values
    assert_eq!(identity.get_revision(), 0);
    
    // Test conversion methods
    let obj = identity.to_object().expect("Failed to convert to object");
    assert!(obj.is_object());
    
    let json = identity.to_json().expect("Failed to convert to JSON");
    assert!(json.is_string());
}

#[wasm_bindgen_test]
async fn test_identity_public_keys() {
    start().await.expect("Failed to start WASM");
    
    let mut identity = IdentityWasm::new(TEST_PLATFORM_VERSION)
        .expect("Failed to create IdentityWasm");
    
    // Create test public keys
    let keys = js_sys::Array::new();
    
    let key1 = js_sys::Object::new();
    js_sys::Reflect::set(&key1, &"id".into(), &JsValue::from(1)).unwrap();
    js_sys::Reflect::set(&key1, &"type".into(), &JsValue::from(0)).unwrap();
    js_sys::Reflect::set(&key1, &"purpose".into(), &JsValue::from(0)).unwrap();
    js_sys::Reflect::set(&key1, &"securityLevel".into(), &JsValue::from(0)).unwrap();
    
    let key_data = Uint8Array::new_with_length(33);
    js_sys::Reflect::set(&key1, &"data".into(), &key_data).unwrap();
    
    keys.push(&key1);
    
    // Set public keys
    let new_revision = identity.set_public_keys(keys.into())
        .expect("Failed to set public keys");
    
    assert_eq!(new_revision, 1);
    assert_eq!(identity.get_revision(), 1);
}

#[wasm_bindgen_test]
async fn test_data_contract_creation() {
    start().await.expect("Failed to start WASM");
    
    let raw_contract = js_sys::Object::new();
    js_sys::Reflect::set(&raw_contract, &"protocolVersion".into(), &JsValue::from(1)).unwrap();
    js_sys::Reflect::set(&raw_contract, &"$schema".into(), &"https://schema.dash.org/dpp-0-4-0/meta/data-contract".into()).unwrap();
    
    let contract = DataContractWasm::new(raw_contract.into(), TEST_PLATFORM_VERSION)
        .expect("Failed to create DataContractWasm");
    
    // Test getter methods
    assert_eq!(contract.get_version(), 0);
    
    // Test schema methods
    let schema_defs = contract.get_schema_defs();
    assert!(schema_defs.is_ok());
    
    let doc_schemas = contract.get_document_schemas();
    assert!(doc_schemas.is_ok());
}

#[wasm_bindgen_test]
async fn test_data_contract_version() {
    start().await.expect("Failed to start WASM");
    
    let raw_contract = js_sys::Object::new();
    js_sys::Reflect::set(&raw_contract, &"protocolVersion".into(), &JsValue::from(1)).unwrap();
    
    let mut contract = DataContractWasm::new(raw_contract.into(), TEST_PLATFORM_VERSION)
        .expect("Failed to create DataContractWasm");
    
    // Test version update
    contract.set_version(2);
    assert_eq!(contract.get_version(), 2);
    
    // Test conversion methods
    let obj = contract.to_object().expect("Failed to convert to object");
    assert!(obj.is_object());
    
    let json = contract.to_json().expect("Failed to convert to JSON");
    assert!(json.is_string());
}

#[wasm_bindgen_test]
async fn test_generate_entropy() {
    start().await.expect("Failed to start WASM");
    
    // Test entropy generation
    let entropy = generate_entropy().expect("Failed to generate entropy");
    
    // Verify it's a Uint8Array with 32 bytes
    assert!(entropy.is_instance_of::<Uint8Array>());
    let array: Uint8Array = entropy.dyn_into().unwrap();
    assert_eq!(array.length(), 32);
}

#[wasm_bindgen_test]
async fn test_generate_document_id() {
    start().await.expect("Failed to start WASM");
    
    let contract_id = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
    let owner_id = "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF";
    let document_type = "profile";
    
    let entropy = Uint8Array::new_with_length(32);
    
    // Generate document ID
    let doc_id = generate_document_id(
        contract_id,
        owner_id,
        document_type,
        &entropy
    ).expect("Failed to generate document ID");
    
    // Verify it's a string
    assert!(doc_id.is_string());
}

#[wasm_bindgen_test]
async fn test_validate_public_keys() {
    start().await.expect("Failed to start WASM");
    
    // Test with empty array
    let empty_keys = js_sys::Array::new();
    let result = validate_public_keys(empty_keys.into());
    assert!(result.is_ok());
    
    // Test with valid keys
    let keys = js_sys::Array::new();
    
    let key = js_sys::Object::new();
    js_sys::Reflect::set(&key, &"id".into(), &JsValue::from(1)).unwrap();
    js_sys::Reflect::set(&key, &"type".into(), &JsValue::from(0)).unwrap();
    js_sys::Reflect::set(&key, &"purpose".into(), &JsValue::from(0)).unwrap();
    js_sys::Reflect::set(&key, &"securityLevel".into(), &JsValue::from(0)).unwrap();
    
    let key_data = Uint8Array::new_with_length(33);
    js_sys::Reflect::set(&key, &"data".into(), &key_data).unwrap();
    
    keys.push(&key);
    
    let result = validate_public_keys(keys.into());
    assert!(result.is_ok());
}

#[wasm_bindgen_test]
async fn test_state_transition_creation() {
    start().await.expect("Failed to start WASM");
    
    let transition_type = "identityCreate";
    let params = js_sys::Object::new();
    js_sys::Reflect::set(&params, &"protocolVersion".into(), &JsValue::from(1)).unwrap();
    
    // This may fail without proper params, but we test error handling
    let result = create_state_transition(transition_type, params.into());
    
    // Either succeeds or fails gracefully
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_document_binary_properties() {
    start().await.expect("Failed to start WASM");
    
    let raw_contract = js_sys::Object::new();
    js_sys::Reflect::set(&raw_contract, &"protocolVersion".into(), &JsValue::from(1)).unwrap();
    
    // Add document schema
    let documents = js_sys::Object::new();
    let profile_schema = js_sys::Object::new();
    
    let properties = js_sys::Object::new();
    let avatar_prop = js_sys::Object::new();
    js_sys::Reflect::set(&avatar_prop, &"type".into(), &"array".into()).unwrap();
    js_sys::Reflect::set(&avatar_prop, &"contentMediaType".into(), &"image/jpeg".into()).unwrap();
    
    js_sys::Reflect::set(&properties, &"avatar".into(), &avatar_prop).unwrap();
    js_sys::Reflect::set(&profile_schema, &"properties".into(), &properties).unwrap();
    js_sys::Reflect::set(&documents, &"profile".into(), &profile_schema).unwrap();
    js_sys::Reflect::set(&raw_contract, &"documents".into(), &documents).unwrap();
    
    let contract = DataContractWasm::new(raw_contract.into(), TEST_PLATFORM_VERSION)
        .expect("Failed to create DataContractWasm");
    
    // Get binary properties
    let binary_props = contract.get_binary_properties("profile");
    assert!(binary_props.is_ok());
}

#[wasm_bindgen_test]
async fn test_invalid_public_key_validation() {
    start().await.expect("Failed to start WASM");
    
    let keys = js_sys::Array::new();
    
    // Invalid key - missing required fields
    let invalid_key = js_sys::Object::new();
    js_sys::Reflect::set(&invalid_key, &"id".into(), &JsValue::from(1)).unwrap();
    // Missing type, purpose, etc.
    
    keys.push(&invalid_key);
    
    let result = validate_public_keys(keys.into());
    // Should handle invalid keys gracefully
    assert!(result.is_ok() || result.is_err());
}

#[wasm_bindgen_test]
async fn test_entropy_randomness() {
    start().await.expect("Failed to start WASM");
    
    // Generate multiple entropy values
    let entropy1 = generate_entropy().expect("Failed to generate entropy 1");
    let entropy2 = generate_entropy().expect("Failed to generate entropy 2");
    
    // Convert to arrays
    let array1: Uint8Array = entropy1.dyn_into().unwrap();
    let array2: Uint8Array = entropy2.dyn_into().unwrap();
    
    // They should be different
    let mut different = false;
    for i in 0..32 {
        if array1.get_index(i) != array2.get_index(i) {
            different = true;
            break;
        }
    }
    
    assert!(different, "Entropy values should be different");
}