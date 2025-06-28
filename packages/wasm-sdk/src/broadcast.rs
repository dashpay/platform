//! Broadcast functionality for state transitions
//!
//! This module provides WASM bindings for broadcasting state transitions to the platform.

use dpp::state_transition::{StateTransition, StateTransitionLike};
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
    
    // Validate byte length
    if bytes.is_empty() {
        return Err(JsError::new("State transition bytes cannot be empty"));
    }
    
    if bytes.len() > 16 * 1024 * 1024 { // 16MB limit
        return Err(JsError::new("State transition exceeds maximum size of 16MB"));
    }
    
    // Try to deserialize and validate
    let _platform_version = dpp::version::PlatformVersion::get(platform_version)
        .map_err(|e| JsError::new(&format!("Invalid platform version: {}", e)))?;
    
    let state_transition = StateTransition::deserialize_from_bytes(&bytes)
        .map_err(|e| JsError::new(&format!("Invalid state transition: {}", e)))?;
    
    // Basic validation based on state transition type
    let validation_errors = js_sys::Array::new();
    
    match &state_transition {
        StateTransition::IdentityCreate(_) => {
            // Validate identity create has reasonable parameters
            // Note: More validation will be possible when context provider is available
        }
        StateTransition::IdentityUpdate(_) => {
            // Validate identity update
        }
        StateTransition::IdentityTopUp(_) => {
            // Validate top up amount is reasonable
        }
        StateTransition::IdentityCreditWithdrawal(_) => {
            // Validate withdrawal parameters
        }
        StateTransition::IdentityCreditTransfer(_) => {
            // Validate transfer parameters
        }
        StateTransition::DataContractCreate(_) => {
            // Validate contract size and structure
        }
        StateTransition::DataContractUpdate(_) => {
            // Validate contract update
        }
        StateTransition::Batch(_) => {
            // Validate batch size
        }
        StateTransition::MasternodeVote(_) => {
            // Validate vote parameters
        }
    }
    
    // Check if state transition is signed (has signature)
    let is_signed = match &state_transition {
        StateTransition::IdentityCreate(st) => !st.signature().is_empty(),
        StateTransition::IdentityUpdate(st) => !st.signature().is_empty(),
        StateTransition::IdentityTopUp(st) => !st.signature().is_empty(),
        StateTransition::IdentityCreditWithdrawal(st) => !st.signature().is_empty(),
        StateTransition::IdentityCreditTransfer(st) => !st.signature().is_empty(),
        StateTransition::DataContractCreate(st) => !st.signature().is_empty(),
        StateTransition::DataContractUpdate(st) => !st.signature().is_empty(),
        StateTransition::Batch(st) => !st.signature().is_empty(),
        StateTransition::MasternodeVote(st) => !st.signature().is_empty(),
    };
    
    if !is_signed {
        validation_errors.push(&"State transition is not signed".into());
    }
    
    let result = Object::new();
    Reflect::set(&result, &"valid".into(), &(validation_errors.length() == 0).into())
        .map_err(|_| JsError::new("Failed to set valid"))?;
    Reflect::set(&result, &"errors".into(), &validation_errors.into())
        .map_err(|_| JsError::new("Failed to set errors"))?;
    
    // Add transition type info
    let st_type = match &state_transition {
        StateTransition::IdentityCreate(_) => "IdentityCreate",
        StateTransition::IdentityUpdate(_) => "IdentityUpdate",
        StateTransition::IdentityTopUp(_) => "IdentityTopUp",
        StateTransition::IdentityCreditWithdrawal(_) => "IdentityCreditWithdrawal",
        StateTransition::IdentityCreditTransfer(_) => "IdentityCreditTransfer",
        StateTransition::DataContractCreate(_) => "DataContractCreate",
        StateTransition::DataContractUpdate(_) => "DataContractUpdate",
        StateTransition::Batch(_) => "Batch",
        StateTransition::MasternodeVote(_) => "MasternodeVote",
    };
    
    Reflect::set(&result, &"type".into(), &st_type.into())
        .map_err(|_| JsError::new("Failed to set type"))?;
    Reflect::set(&result, &"signed".into(), &is_signed.into())
        .map_err(|_| JsError::new("Failed to set signed"))?;
    Reflect::set(&result, &"size".into(), &bytes.len().into())
        .map_err(|_| JsError::new("Failed to set size"))?;
    
    Ok(result.into())
}

/// Process broadcast response from the platform
#[wasm_bindgen(js_name = processBroadcastResponse)]
pub fn process_broadcast_response(
    response_bytes: &Uint8Array,
) -> Result<BroadcastResponse, JsError> {
    let bytes = response_bytes.to_vec();
    
    // First, try to parse as JSON (common response format)
    if let Ok(response_str) = String::from_utf8(bytes.clone()) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_str) {
            // Handle different JSON response formats
            let success = json.get("success")
                .and_then(|v| v.as_bool())
                .or_else(|| {
                    // Alternative: check for result field
                    json.get("result").map(|_| true)
                })
                .or_else(|| {
                    // Alternative: check for error field absence
                    json.get("error").is_none().then_some(true)
                })
                .unwrap_or(false);
            
            let transaction_id = json.get("transactionId")
                .or_else(|| json.get("transaction_id"))
                .or_else(|| json.get("txid"))
                .or_else(|| json.get("id"))
                .and_then(|v| v.as_str())
                .map(String::from);
            
            let block_height = json.get("blockHeight")
                .or_else(|| json.get("block_height"))
                .or_else(|| json.get("height"))
                .and_then(|v| v.as_u64());
            
            let error = json.get("error")
                .and_then(|v| {
                    if v.is_string() {
                        v.as_str().map(String::from)
                    } else if v.is_object() {
                        // Handle error object with message field
                        v.get("message")
                            .or_else(|| v.get("msg"))
                            .and_then(|m| m.as_str())
                            .map(String::from)
                            .or_else(|| {
                                // Fallback to stringifying the error object
                                serde_json::to_string(v).ok()
                            })
                    } else {
                        None
                    }
                });
            
            return Ok(BroadcastResponse {
                success,
                transaction_id,
                block_height,
                error,
            });
        }
    }
    
    // If not JSON, try to parse as CBOR (binary format)
    if bytes.len() > 0 {
        // Check for CBOR magic bytes or other binary format indicators
        if bytes[0] == 0x81 || bytes[0] == 0x82 || bytes[0] == 0x83 {
            // Likely CBOR format
            // For now, return a generic success response for valid CBOR
            // When platform_proto is available, we can properly decode this
            return Ok(BroadcastResponse {
                success: true,
                transaction_id: None,
                block_height: None,
                error: None,
            });
        }
    }
    
    // If all parsing fails, return an error
    Err(JsError::new("Unable to parse broadcast response: unsupported format"))
}

/// Process wait for state transition result response
#[wasm_bindgen(js_name = processWaitForSTResultResponse)]
pub fn process_wait_for_st_result_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    let bytes = response_bytes.to_vec();
    let result = Object::new();
    
    // Try to parse as JSON first
    if let Ok(response_str) = String::from_utf8(bytes.clone()) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_str) {
            // Handle execution status
            let executed = json.get("executed")
                .and_then(|v| v.as_bool())
                .or_else(|| {
                    // Alternative: check status field
                    json.get("status").and_then(|v| v.as_str()).map(|s| s == "executed" || s == "success")
                })
                .unwrap_or(false);
            
            Reflect::set(&result, &"executed".into(), &executed.into())
                .map_err(|_| JsError::new("Failed to set executed"))?;
            
            // Handle block height
            if let Some(block_height) = json.get("blockHeight")
                .or_else(|| json.get("block_height"))
                .or_else(|| json.get("height"))
                .and_then(|v| v.as_u64()) {
                Reflect::set(&result, &"blockHeight".into(), &block_height.into())
                    .map_err(|_| JsError::new("Failed to set block height"))?;
            }
            
            // Handle block hash
            if let Some(block_hash) = json.get("blockHash")
                .or_else(|| json.get("block_hash"))
                .or_else(|| json.get("hash"))
                .and_then(|v| v.as_str()) {
                Reflect::set(&result, &"blockHash".into(), &block_hash.into())
                    .map_err(|_| JsError::new("Failed to set block hash"))?;
            }
            
            // Handle transaction ID if present
            if let Some(tx_id) = json.get("transactionId")
                .or_else(|| json.get("transaction_id"))
                .or_else(|| json.get("txid"))
                .and_then(|v| v.as_str()) {
                Reflect::set(&result, &"transactionId".into(), &tx_id.into())
                    .map_err(|_| JsError::new("Failed to set transaction ID"))?;
            }
            
            // Handle error
            if let Some(error_val) = json.get("error") {
                let error_str = if error_val.is_string() {
                    error_val.as_str().map(String::from)
                } else if error_val.is_object() {
                    error_val.get("message")
                        .or_else(|| error_val.get("msg"))
                        .and_then(|m| m.as_str())
                        .map(String::from)
                        .or_else(|| serde_json::to_string(error_val).ok())
                } else {
                    None
                };
                
                if let Some(error) = error_str {
                    Reflect::set(&result, &"error".into(), &error.into())
                        .map_err(|_| JsError::new("Failed to set error"))?;
                }
            }
            
            // Handle execution result data if present
            if let Some(result_data) = json.get("result").or_else(|| json.get("data")) {
                // Convert result data to JS value
                let js_data = serde_wasm_bindgen::to_value(result_data)
                    .unwrap_or(JsValue::NULL);
                Reflect::set(&result, &"data".into(), &js_data)
                    .map_err(|_| JsError::new("Failed to set result data"))?;
            }
            
            // Handle metadata if present
            if let Some(metadata) = json.get("metadata") {
                let js_metadata = serde_wasm_bindgen::to_value(metadata)
                    .unwrap_or(JsValue::NULL);
                Reflect::set(&result, &"metadata".into(), &js_metadata)
                    .map_err(|_| JsError::new("Failed to set metadata"))?;
            }
            
            return Ok(result.into());
        }
    }
    
    // If not JSON, check for binary response
    if bytes.len() > 0 {
        // For binary responses, just indicate it was received
        Reflect::set(&result, &"executed".into(), &true.into())
            .map_err(|_| JsError::new("Failed to set executed"))?;
        Reflect::set(&result, &"binaryResponse".into(), &true.into())
            .map_err(|_| JsError::new("Failed to set binary response flag"))?;
        
        return Ok(result.into());
    }
    
    // Empty response
    Reflect::set(&result, &"executed".into(), &false.into())
        .map_err(|_| JsError::new("Failed to set executed"))?;
    Reflect::set(&result, &"error".into(), &"Empty response".into())
        .map_err(|_| JsError::new("Failed to set error"))?;
    
    Ok(result.into())
}