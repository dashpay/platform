//! Broadcast functionality for state transitions
//!
//! This module provides WASM bindings for broadcasting state transitions to the platform.

use crate::sdk::WasmSdk;
use dpp::state_transition::StateTransition;
use dpp::serialization::PlatformDeserializable;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Object, Reflect, Uint8Array};

/// Broadcast options
#[wasm_bindgen]
pub struct BroadcastOptions {
    wait_for_confirmation: bool,
    retry_count: u32,
    timeout_ms: u32,
}

#[wasm_bindgen]
impl BroadcastOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BroadcastOptions {
        BroadcastOptions {
            wait_for_confirmation: true,
            retry_count: 3,
            timeout_ms: 60000, // 60 seconds
        }
    }

    #[wasm_bindgen(js_name = setWaitForConfirmation)]
    pub fn set_wait_for_confirmation(&mut self, wait: bool) {
        self.wait_for_confirmation = wait;
    }

    #[wasm_bindgen(js_name = setRetryCount)]
    pub fn set_retry_count(&mut self, count: u32) {
        self.retry_count = count;
    }

    #[wasm_bindgen(js_name = setTimeoutMs)]
    pub fn set_timeout_ms(&mut self, timeout: u32) {
        self.timeout_ms = timeout;
    }
    
    #[wasm_bindgen(getter, js_name = waitForConfirmation)]
    pub fn wait_for_confirmation(&self) -> bool {
        self.wait_for_confirmation
    }
    
    #[wasm_bindgen(getter, js_name = retryCount)]
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }
    
    #[wasm_bindgen(getter, js_name = timeoutMs)]
    pub fn timeout_ms(&self) -> u32 {
        self.timeout_ms
    }
}

/// Response from broadcasting a state transition
#[wasm_bindgen]
pub struct BroadcastResponse {
    success: bool,
    transaction_id: Option<String>,
    block_height: Option<u64>,
    error: Option<String>,
}

#[wasm_bindgen]
impl BroadcastResponse {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen(getter, js_name = transactionId)]
    pub fn transaction_id(&self) -> Option<String> {
        self.transaction_id.clone()
    }

    #[wasm_bindgen(getter, js_name = blockHeight)]
    pub fn block_height(&self) -> Option<u64> {
        self.block_height
    }

    #[wasm_bindgen(getter)]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"success".into(), &self.success.into())
            .map_err(|_| JsError::new("Failed to set success"))?;
        
        if let Some(ref tx_id) = self.transaction_id {
            Reflect::set(&obj, &"transactionId".into(), &tx_id.clone().into())
                .map_err(|_| JsError::new("Failed to set transaction ID"))?;
        }
        
        if let Some(height) = self.block_height {
            Reflect::set(&obj, &"blockHeight".into(), &height.into())
                .map_err(|_| JsError::new("Failed to set block height"))?;
        }
        
        if let Some(ref err) = self.error {
            Reflect::set(&obj, &"error".into(), &err.clone().into())
                .map_err(|_| JsError::new("Failed to set error"))?;
        }
        
        Ok(obj.into())
    }
}

/// Calculate the hash of a state transition
#[wasm_bindgen(js_name = calculateStateTransitionHash)]
pub fn calculate_state_transition_hash(
    state_transition_bytes: &Uint8Array,
) -> Result<String, JsError> {
    let bytes = state_transition_bytes.to_vec();
    
    // Calculate SHA256 hash of the state transition
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    
    // Return hex string
    Ok(hex::encode(result))
}

/// Validate a state transition before broadcasting
#[wasm_bindgen(js_name = validateStateTransition)]
pub fn validate_state_transition(
    state_transition_bytes: &Uint8Array,
    platform_version: u32,
) -> Result<JsValue, JsError> {
    let bytes = state_transition_bytes.to_vec();
    
    // Try to deserialize and validate
    let platform_version = dpp::version::PlatformVersion::get(platform_version)
        .map_err(|e| JsError::new(&format!("Invalid platform version: {}", e)))?;
    
    let _state_transition = StateTransition::deserialize_from_bytes(&bytes)
        .map_err(|e| JsError::new(&format!("Invalid state transition: {}", e)))?;
    
    // TODO: Add more validation when we have context provider working
    
    let result = Object::new();
    Reflect::set(&result, &"valid".into(), &true.into())
        .map_err(|_| JsError::new("Failed to set valid"))?;
    Reflect::set(&result, &"errors".into(), &js_sys::Array::new().into())
        .map_err(|_| JsError::new("Failed to set errors"))?;
    
    Ok(result.into())
}

/// Process broadcast response from the platform
#[wasm_bindgen(js_name = processBroadcastResponse)]
pub fn process_broadcast_response(
    response_bytes: &Uint8Array,
) -> Result<BroadcastResponse, JsError> {
    let bytes = response_bytes.to_vec();
    
    // TODO: Implement actual response parsing when we have platform_proto types
    // For now, parse a simple JSON response
    let response_str = String::from_utf8(bytes)
        .map_err(|e| JsError::new(&format!("Invalid UTF-8 in response: {}", e)))?;
    
    let json: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| JsError::new(&format!("Invalid JSON response: {}", e)))?;
    
    Ok(BroadcastResponse {
        success: json.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
        transaction_id: json.get("transactionId").and_then(|v| v.as_str()).map(String::from),
        block_height: json.get("blockHeight").and_then(|v| v.as_u64()),
        error: json.get("error").and_then(|v| v.as_str()).map(String::from),
    })
}

/// Process wait for state transition result response
#[wasm_bindgen(js_name = processWaitForSTResultResponse)]
pub fn process_wait_for_st_result_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    let bytes = response_bytes.to_vec();
    
    // TODO: Implement actual response parsing
    let response_str = String::from_utf8(bytes)
        .map_err(|e| JsError::new(&format!("Invalid UTF-8 in response: {}", e)))?;
    
    let json: serde_json::Value = serde_json::from_str(&response_str)
        .map_err(|e| JsError::new(&format!("Invalid JSON response: {}", e)))?;
    
    let result = Object::new();
    
    if let Some(executed) = json.get("executed").and_then(|v| v.as_bool()) {
        Reflect::set(&result, &"executed".into(), &executed.into())
            .map_err(|_| JsError::new("Failed to set executed"))?;
    }
    
    if let Some(block_height) = json.get("blockHeight").and_then(|v| v.as_u64()) {
        Reflect::set(&result, &"blockHeight".into(), &block_height.into())
            .map_err(|_| JsError::new("Failed to set block height"))?;
    }
    
    if let Some(block_hash) = json.get("blockHash").and_then(|v| v.as_str()) {
        Reflect::set(&result, &"blockHash".into(), &block_hash.into())
            .map_err(|_| JsError::new("Failed to set block hash"))?;
    }
    
    if let Some(error) = json.get("error").and_then(|v| v.as_str()) {
        Reflect::set(&result, &"error".into(), &error.into())
            .map_err(|_| JsError::new("Failed to set error"))?;
    }
    
    Ok(result.into())
}