//! Test utilities and helpers

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;

/// Create a mock DAPI response
pub fn mock_dapi_response(data: JsValue) -> Object {
    let response = Object::new();
    Reflect::set(&response, &"data".into(), &data).unwrap();
    Reflect::set(&response, &"metadata".into(), &mock_metadata()).unwrap();
    response
}

/// Create mock metadata
pub fn mock_metadata() -> Object {
    let metadata = Object::new();
    Reflect::set(&metadata, &"height".into(), &12345.into()).unwrap();
    Reflect::set(&metadata, &"core_chain_locked_height".into(), &12340.into()).unwrap();
    Reflect::set(&metadata, &"time_ms".into(), &js_sys::Date::now().into()).unwrap();
    Reflect::set(&metadata, &"protocol_version".into(), &1.into()).unwrap();
    metadata
}

/// Create a mock identity object
pub fn mock_identity(id: &str, balance: u64) -> Object {
    let identity = Object::new();
    Reflect::set(&identity, &"id".into(), &id.into()).unwrap();
    Reflect::set(&identity, &"balance".into(), &balance.into()).unwrap();
    Reflect::set(&identity, &"revision".into(), &0.into()).unwrap();

    let public_keys = js_sys::Array::new();
    public_keys.push(&mock_public_key(0));
    Reflect::set(&identity, &"publicKeys".into(), &public_keys).unwrap();

    identity
}

/// Create a mock public key
pub fn mock_public_key(id: u32) -> Object {
    let key = Object::new();
    Reflect::set(&key, &"id".into(), &id.into()).unwrap();
    Reflect::set(&key, &"type".into(), &"ECDSA_SECP256K1".into()).unwrap();
    Reflect::set(&key, &"purpose".into(), &"AUTHENTICATION".into()).unwrap();
    Reflect::set(&key, &"security_level".into(), &"MASTER".into()).unwrap();
    Reflect::set(&key, &"read_only".into(), &false.into()).unwrap();

    // Mock public key data (33 bytes for compressed secp256k1)
    let key_data = js_sys::Uint8Array::new_with_length(33);
    key_data.set_index(0, 0x02); // Compressed key prefix
    for i in 1..33 {
        key_data.set_index(i, i as u8);
    }
    Reflect::set(&key, &"data".into(), &key_data).unwrap();

    key
}

/// Create a mock data contract
pub fn mock_data_contract(id: &str, owner_id: &str) -> Object {
    let contract = Object::new();
    Reflect::set(&contract, &"id".into(), &id.into()).unwrap();
    Reflect::set(&contract, &"owner_id".into(), &owner_id.into()).unwrap();
    Reflect::set(&contract, &"version".into(), &1.into()).unwrap();
    Reflect::set(&contract, &"schema".into(), &mock_contract_schema()).unwrap();
    contract
}

/// Create a mock contract schema
pub fn mock_contract_schema() -> Object {
    let schema = Object::new();

    // Add a simple document type
    let message_type = Object::new();
    Reflect::set(&message_type, &"type".into(), &"object".into()).unwrap();

    let properties = Object::new();

    let text_prop = Object::new();
    Reflect::set(&text_prop, &"type".into(), &"string".into()).unwrap();
    Reflect::set(&properties, &"text".into(), &text_prop).unwrap();

    let timestamp_prop = Object::new();
    Reflect::set(&timestamp_prop, &"type".into(), &"integer".into()).unwrap();
    Reflect::set(&properties, &"timestamp".into(), &timestamp_prop).unwrap();

    Reflect::set(&message_type, &"properties".into(), &properties).unwrap();
    Reflect::set(&schema, &"message".into(), &message_type).unwrap();

    schema
}

/// Create a mock document
pub fn mock_document(id: &str, owner_id: &str, doc_type: &str) -> Object {
    let document = Object::new();
    Reflect::set(&document, &"$id".into(), &id.into()).unwrap();
    Reflect::set(&document, &"$ownerId".into(), &owner_id.into()).unwrap();
    Reflect::set(&document, &"$type".into(), &doc_type.into()).unwrap();
    Reflect::set(&document, &"$revision".into(), &1.into()).unwrap();
    Reflect::set(&document, &"$createdAt".into(), &js_sys::Date::now().into()).unwrap();
    document
}

/// Create a mock state transition result
pub fn mock_state_transition_result(success: bool) -> Object {
    let result = Object::new();
    Reflect::set(&result, &"success".into(), &success.into()).unwrap();

    if success {
        Reflect::set(&result, &"fee".into(), &1000.into()).unwrap();
        Reflect::set(&result, &"block_height".into(), &12346.into()).unwrap();
    } else {
        let error = Object::new();
        Reflect::set(&error, &"code".into(), &4000.into()).unwrap();
        Reflect::set(&error, &"message".into(), &"Mock error".into()).unwrap();
        Reflect::set(&result, &"error".into(), &error).unwrap();
    }

    result
}

/// Create a test asset lock proof
pub fn create_test_asset_lock_proof() -> Vec<u8> {
    // Create a minimal valid asset lock proof structure
    let mut proof = Vec::new();

    // Version byte
    proof.push(0x01);

    // Type (instant lock)
    proof.push(0x00);

    // Mock transaction data (simplified)
    proof.extend_from_slice(&[0u8; 32]); // Mock tx hash
    proof.extend_from_slice(&[0u8; 4]); // Output index

    // Mock instant lock data
    proof.extend_from_slice(&[0u8; 32]); // Mock instant lock hash

    proof
}

/// Generate a deterministic test key pair
pub fn generate_test_key_pair(seed: u8) -> (Vec<u8>, Vec<u8>) {
    let mut private_key = vec![seed; 32];
    let mut public_key = vec![0x02]; // Compressed public key prefix
    public_key.extend_from_slice(&[seed; 32]);

    (private_key, public_key)
}

/// Console log helper for tests
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

/// Macro for console logging in tests
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => {
        $crate::test_utils::log(&format!($($t)*))
    };
}
