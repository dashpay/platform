//! Data contract tests

mod common;
use common::*;
use wasm_bindgen_test::*;
use wasm_sdk::{
    contract_history::{
        fetch_contract_history, fetch_contract_versions, get_schema_changes,
        check_contract_updates, get_migration_guide
    },
    fetch::{fetch_data_contract, FetchOptions},
    fetch_unproved::fetch_data_contract_unproved,
    nonce::get_identity_contract_nonce,
    state_transitions::data_contract::{create_data_contract, update_data_contract},
};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_create_data_contract() {
    let owner_id = test_identity_id();
    let identity_nonce = 1u64;
    let signature_key_id = 0u32;
    
    // Create contract definition
    let contract_def = js_sys::Object::new();
    let documents = js_sys::Object::new();
    
    // Define a simple document type
    let message_doc = js_sys::Object::new();
    js_sys::Reflect::set(&message_doc, &"type".into(), &"object".into()).unwrap();
    
    let properties = js_sys::Object::new();
    let text_prop = js_sys::Object::new();
    js_sys::Reflect::set(&text_prop, &"type".into(), &"string".into()).unwrap();
    js_sys::Reflect::set(&properties, &"text".into(), &text_prop).unwrap();
    
    js_sys::Reflect::set(&message_doc, &"properties".into(), &properties).unwrap();
    js_sys::Reflect::set(&message_doc, &"additionalProperties".into(), &false.into()).unwrap();
    
    js_sys::Reflect::set(&documents, &"message".into(), &message_doc).unwrap();
    js_sys::Reflect::set(&contract_def, &"documents".into(), &documents).unwrap();
    
    let result = create_data_contract(
        &owner_id,
        contract_def.into(),
        identity_nonce,
        signature_key_id
    );
    
    assert!(result.is_ok(), "Should create data contract state transition");
    assert!(!result.unwrap().is_empty(), "State transition should not be empty");
}

#[wasm_bindgen_test]
async fn test_update_data_contract() {
    let contract_id = test_contract_id();
    let owner_id = test_identity_id();
    let contract_nonce = 1u64;
    let signature_key_id = 0u32;
    
    let updated_def = js_sys::Object::new();
    
    let result = update_data_contract(
        &contract_id,
        &owner_id,
        updated_def.into(),
        contract_nonce,
        signature_key_id
    );
    
    assert!(result.is_ok(), "Should create update data contract state transition");
}

#[wasm_bindgen_test]
async fn test_fetch_data_contract() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    // Test basic fetch
    let result = fetch_data_contract(&sdk, &contract_id, None).await;
    assert!(result.is_ok(), "Should fetch data contract");
    
    // Test fetch with options
    let options = FetchOptions::new();
    let result_with_options = fetch_data_contract(&sdk, &contract_id, Some(options)).await;
    assert!(result_with_options.is_ok(), "Should fetch data contract with options");
}

#[wasm_bindgen_test]
async fn test_fetch_data_contract_unproved() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    let result = fetch_data_contract_unproved(&sdk, &contract_id, None).await;
    assert!(result.is_ok(), "Should fetch data contract without proof");
}

#[wasm_bindgen_test]
async fn test_contract_nonce() {
    let sdk = setup_test_sdk().await;
    let identity_id = test_identity_id();
    let contract_id = test_contract_id();
    
    let nonce = get_identity_contract_nonce(&sdk, &identity_id, &contract_id, false).await;
    assert!(nonce.is_ok(), "Should get identity contract nonce");
}

#[wasm_bindgen_test]
async fn test_contract_history() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    // Test fetch history
    let history = fetch_contract_history(&sdk, &contract_id, None, None, None).await;
    assert!(history.is_ok(), "Should fetch contract history");
    
    let entries = history.unwrap();
    assert!(entries.length() >= 0, "Should return history array");
}

#[wasm_bindgen_test]
async fn test_contract_versions() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    let versions = fetch_contract_versions(&sdk, &contract_id).await;
    assert!(versions.is_ok(), "Should fetch contract versions");
    
    let version_list = versions.unwrap();
    assert!(version_list.length() >= 0, "Should return versions array");
}

#[wasm_bindgen_test]
async fn test_schema_changes() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    let changes = get_schema_changes(&sdk, &contract_id, 1, 2).await;
    assert!(changes.is_ok(), "Should get schema changes");
    
    // Test invalid version range
    let invalid_changes = get_schema_changes(&sdk, &contract_id, 2, 1).await;
    assert!(invalid_changes.is_err(), "Should fail with invalid version range");
}

#[wasm_bindgen_test]
async fn test_check_contract_updates() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    let has_updates = check_contract_updates(&sdk, &contract_id, 1).await;
    assert!(has_updates.is_ok(), "Should check for contract updates");
}

#[wasm_bindgen_test]
async fn test_migration_guide() {
    let sdk = setup_test_sdk().await;
    let contract_id = test_contract_id();
    
    let guide = get_migration_guide(&sdk, &contract_id, 1, 2).await;
    assert!(guide.is_ok(), "Should get migration guide");
    
    let guide_obj = guide.unwrap();
    assert_not_null(&guide_obj);
    
    // Test invalid version range
    let invalid_guide = get_migration_guide(&sdk, &contract_id, 2, 1).await;
    assert!(invalid_guide.is_err(), "Should fail with invalid version range");
}