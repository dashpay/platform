//! Request/Response Serialization for JavaScript Transport
//!
//! This module provides serialization and deserialization functions for platform
//! requests and responses. JavaScript will handle the actual network transport.

use dpp::prelude::*;
use js_sys::Uint8Array;
use platform_value::Identifier;
use wasm_bindgen::prelude::*;

/// Serialize a GetIdentity request
#[wasm_bindgen(js_name = serializeGetIdentityRequest)]
pub fn serialize_get_identity_request(
    identity_id: &str,
    prove: bool,
) -> Result<Uint8Array, JsError> {
    let id = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    // Create request object
    let request = serde_json::json!({
        "id": id.to_string(platform_value::string_encoding::Encoding::Base58),
        "prove": prove,
    });
    
    let bytes = serde_json::to_vec(&request)
        .map_err(|e| JsError::new(&format!("Failed to serialize request: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Deserialize a GetIdentity response
#[wasm_bindgen(js_name = deserializeGetIdentityResponse)]
pub fn deserialize_get_identity_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    use crate::dpp::IdentityWasm;
    use dpp::identity::Identity;
    use dpp::serialization::PlatformDeserializable;
    
    let bytes = response_bytes.to_vec();
    
    // Try to parse as JSON response first (from DAPI)
    if let Ok(json_response) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        // Check if it's an error response
        if let Some(error) = json_response.get("error") {
            return Err(JsError::new(&format!("DAPI error: {:?}", error)));
        }
        
        // Extract identity data
        if let Some(identity_data) = json_response.get("identity") {
            return serde_wasm_bindgen::to_value(identity_data)
                .map_err(|e| JsError::new(&format!("Failed to convert identity to JS value: {}", e)));
        }
    }
    
    // If not JSON, try to deserialize as raw identity bytes
    let platform_version = platform_version::version::PlatformVersion::latest();
    match Identity::deserialize_from_bytes(&bytes) {
        Ok(identity) => {
            let identity_wasm = IdentityWasm::from(identity);
            // Convert to JSON and then to JS value
            let identity_json = serde_json::json!({
                "id": identity_wasm.id(),
                "balance": identity_wasm.get_balance(),
                "revision": identity_wasm.revision(),
            });
            serde_wasm_bindgen::to_value(&identity_json)
                .map_err(|e| JsError::new(&format!("Failed to convert identity to JS value: {}", e)))
        }
        Err(e) => Err(JsError::new(&format!("Failed to deserialize identity: {}", e))),
    }
}

/// Serialize a GetDataContract request
#[wasm_bindgen(js_name = serializeGetDataContractRequest)]
pub fn serialize_get_data_contract_request(
    contract_id: &str,
    prove: bool,
) -> Result<Uint8Array, JsError> {
    let id = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    let request = serde_json::json!({
        "id": id.to_string(platform_value::string_encoding::Encoding::Base58),
        "prove": prove,
    });
    
    let bytes = serde_json::to_vec(&request)
        .map_err(|e| JsError::new(&format!("Failed to serialize request: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Deserialize a GetDataContract response
#[wasm_bindgen(js_name = deserializeGetDataContractResponse)]
pub fn deserialize_get_data_contract_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    use crate::dpp::DataContractWasm;
    use dpp::data_contract::DataContract;
    use dpp::serialization::PlatformLimitDeserializableFromVersionedStructure;
    
    let bytes = response_bytes.to_vec();
    
    // Try to parse as JSON response first (from DAPI)
    if let Ok(json_response) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        // Check if it's an error response
        if let Some(error) = json_response.get("error") {
            return Err(JsError::new(&format!("DAPI error: {:?}", error)));
        }
        
        // Extract data contract
        if let Some(contract_data) = json_response.get("dataContract") {
            return serde_wasm_bindgen::to_value(contract_data)
                .map_err(|e| JsError::new(&format!("Failed to convert data contract to JS value: {}", e)));
        }
    }
    
    // If not JSON, try to deserialize as raw contract bytes
    let platform_version = platform_version::version::PlatformVersion::latest();
    match DataContract::versioned_limit_deserialize(&bytes, platform_version) {
        Ok(contract) => {
            let contract_wasm = DataContractWasm::from(contract);
            // Convert to JSON and then to JS value
            let contract_json = serde_json::json!({
                "id": contract_wasm.id(),
                "version": contract_wasm.version(),
                "ownerId": contract_wasm.owner_id(),
            });
            serde_wasm_bindgen::to_value(&contract_json)
                .map_err(|e| JsError::new(&format!("Failed to convert data contract to JS value: {}", e)))
        }
        Err(e) => Err(JsError::new(&format!("Failed to deserialize data contract: {}", e))),
    }
}

/// Serialize a BroadcastStateTransition request
#[wasm_bindgen(js_name = serializeBroadcastRequest)]
pub fn serialize_broadcast_request(
    state_transition_bytes: &Uint8Array,
) -> Result<Uint8Array, JsError> {
    let st_bytes = state_transition_bytes.to_vec();
    
    use base64::{Engine as _, engine::general_purpose};
    
    let request = serde_json::json!({
        "stateTransition": general_purpose::STANDARD.encode(&st_bytes),
    });
    
    let bytes = serde_json::to_vec(&request)
        .map_err(|e| JsError::new(&format!("Failed to serialize request: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Deserialize a BroadcastStateTransition response
#[wasm_bindgen(js_name = deserializeBroadcastResponse)]
pub fn deserialize_broadcast_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    let bytes = response_bytes.to_vec();
    
    // Parse JSON response from DAPI
    let json_response: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| JsError::new(&format!("Failed to parse broadcast response: {}", e)))?;
    
    // Check if it's an error response
    if let Some(error) = json_response.get("error") {
        return Err(JsError::new(&format!("Broadcast error: {:?}", error)));
    }
    
    // Extract relevant fields
    let response = if let Some(result) = json_response.get("result") {
        serde_json::json!({
            "success": true,
            "transactionId": result.get("transactionId").and_then(|v| v.as_str()).unwrap_or(""),
            "blockHeight": result.get("blockHeight").and_then(|v| v.as_u64()).unwrap_or(0),
            "blockHash": result.get("blockHash").and_then(|v| v.as_str()).unwrap_or(""),
        })
    } else {
        serde_json::json!({
            "success": false,
            "error": "Invalid broadcast response format"
        })
    };
    
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsError::new(&format!("Failed to convert to JS value: {}", e)))
}

/// Serialize a GetIdentityNonce request
#[wasm_bindgen(js_name = serializeGetIdentityNonceRequest)]
pub fn serialize_get_identity_nonce_request(
    identity_id: &str,
    prove: bool,
) -> Result<Uint8Array, JsError> {
    let id = Identifier::from_string(
        identity_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid identity ID: {}", e)))?;
    
    let request = serde_json::json!({
        "identityId": id.to_string(platform_value::string_encoding::Encoding::Base58),
        "prove": prove,
    });
    
    let bytes = serde_json::to_vec(&request)
        .map_err(|e| JsError::new(&format!("Failed to serialize request: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Deserialize a GetIdentityNonce response
#[wasm_bindgen(js_name = deserializeGetIdentityNonceResponse)]
pub fn deserialize_get_identity_nonce_response(
    response_bytes: &Uint8Array,
) -> Result<u64, JsError> {
    let bytes = response_bytes.to_vec();
    
    // Parse the response
    let json_response: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| JsError::new(&format!("Failed to parse nonce response: {}", e)))?;
    
    // Check for error
    if let Some(error) = json_response.get("error") {
        return Err(JsError::new(&format!("DAPI error: {:?}", error)));
    }
    
    // Extract nonce from response
    let nonce = json_response.get("nonce")
        .or_else(|| json_response.get("identityNonce"))
        .or_else(|| json_response.get("revision"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| JsError::new("Missing or invalid nonce in response"))?;
    
    Ok(nonce)
}

/// Serialize a WaitForStateTransitionResult request
#[wasm_bindgen(js_name = serializeWaitForStateTransitionRequest)]
pub fn serialize_wait_for_state_transition_request(
    state_transition_hash: &str,
    prove: bool,
) -> Result<Uint8Array, JsError> {
    let request = serde_json::json!({
        "stateTransitionHash": state_transition_hash,
        "prove": prove,
    });
    
    let bytes = serde_json::to_vec(&request)
        .map_err(|e| JsError::new(&format!("Failed to serialize request: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Deserialize a WaitForStateTransitionResult response
#[wasm_bindgen(js_name = deserializeWaitForStateTransitionResponse)]
pub fn deserialize_wait_for_state_transition_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    let bytes = response_bytes.to_vec();
    
    // Parse the response
    let json_response: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| JsError::new(&format!("Failed to parse wait response: {}", e)))?;
    
    // Check for error
    if let Some(error) = json_response.get("error") {
        return Err(JsError::new(&format!("DAPI error: {:?}", error)));
    }
    
    // Extract the result
    let result = if let Some(result_obj) = json_response.get("result") {
        serde_json::json!({
            "executed": result_obj.get("executed").and_then(|v| v.as_bool()).unwrap_or(false),
            "blockHeight": result_obj.get("blockHeight").and_then(|v| v.as_u64()).unwrap_or(0),
            "blockHash": result_obj.get("blockHash").and_then(|v| v.as_str()).unwrap_or(""),
            "error": result_obj.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
            "metadata": result_obj.get("metadata"),
        })
    } else {
        // Fallback for different response format
        serde_json::json!({
            "executed": json_response.get("executed").and_then(|v| v.as_bool()).unwrap_or(false),
            "blockHeight": json_response.get("blockHeight").and_then(|v| v.as_u64()).unwrap_or(0),
            "blockHash": json_response.get("blockHash").and_then(|v| v.as_str()).unwrap_or(""),
            "error": json_response.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    };
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to convert to JS value: {}", e)))
}

/// Serialize document query parameters
#[wasm_bindgen(js_name = serializeDocumentQuery)]
pub fn serialize_document_query(
    contract_id: &str,
    document_type: &str,
    where_clause: &JsValue,
    order_by: &JsValue,
    limit: Option<u32>,
    start_after: Option<String>,
    prove: bool,
) -> Result<Uint8Array, JsError> {
    let contract_id = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;
    
    let mut request = serde_json::json!({
        "contractId": contract_id.to_string(platform_value::string_encoding::Encoding::Base58),
        "documentType": document_type,
        "prove": prove,
    });
    
    // Add optional parameters
    if !where_clause.is_null() && !where_clause.is_undefined() {
        let where_obj = serde_wasm_bindgen::from_value::<serde_json::Value>(where_clause.clone())
            .map_err(|e| JsError::new(&format!("Invalid where clause: {}", e)))?;
        request["where"] = where_obj;
    }
    
    if !order_by.is_null() && !order_by.is_undefined() {
        let order_obj = serde_wasm_bindgen::from_value::<serde_json::Value>(order_by.clone())
            .map_err(|e| JsError::new(&format!("Invalid order by: {}", e)))?;
        request["orderBy"] = order_obj;
    }
    
    if let Some(limit) = limit {
        request["limit"] = serde_json::json!(limit);
    }
    
    if let Some(start_after) = start_after {
        request["startAfter"] = serde_json::json!(start_after);
    }
    
    let bytes = serde_json::to_vec(&request)
        .map_err(|e| JsError::new(&format!("Failed to serialize request: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Deserialize document query response
#[wasm_bindgen(js_name = deserializeDocumentQueryResponse)]
pub fn deserialize_document_query_response(
    response_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    let bytes = response_bytes.to_vec();
    
    // Parse the response
    let json_response: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| JsError::new(&format!("Failed to parse document query response: {}", e)))?;
    
    // Check for error
    if let Some(error) = json_response.get("error") {
        return Err(JsError::new(&format!("DAPI error: {:?}", error)));
    }
    
    // Extract documents and metadata
    let result = if let Some(result_obj) = json_response.get("result") {
        // Handle result wrapper
        serde_json::json!({
            "documents": result_obj.get("documents").unwrap_or(&serde_json::json!([])),
            "startAfter": result_obj.get("startAfter"),
            "metadata": result_obj.get("metadata").unwrap_or(&serde_json::json!({
                "height": 0,
                "timeMs": 0,
                "protocolVersion": 1
            }))
        })
    } else {
        // Direct format
        serde_json::json!({
            "documents": json_response.get("documents").unwrap_or(&serde_json::json!([])),
            "startAfter": json_response.get("startAfter"),
            "metadata": json_response.get("metadata").unwrap_or(&serde_json::json!({
                "height": 0,
                "timeMs": 0,
                "protocolVersion": 1
            }))
        })
    };
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to convert to JS value: {}", e)))
}

/// Prepare a state transition for broadcast
#[wasm_bindgen(js_name = prepareStateTransitionForBroadcast)]
pub fn prepare_state_transition_for_broadcast(
    state_transition_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    use dpp::state_transition::StateTransition;
    use dpp::serialization::PlatformDeserializable;
    use crate::state_transitions::serialization::calculate_state_transition_id;
    
    let bytes = state_transition_bytes.to_vec();
    let platform_version = platform_version::version::PlatformVersion::latest();
    
    // Deserialize to validate
    let _state_transition = StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
        .map_err(|e| JsError::new(&format!("Invalid state transition: {}", e)))?;
    
    // Calculate hash for tracking
    let hash = calculate_state_transition_id(state_transition_bytes)?;
    
    use base64::{Engine as _, engine::general_purpose};
    
    let result = serde_json::json!({
        "bytes": general_purpose::STANDARD.encode(&bytes),
        "hash": hash,
        "size": bytes.len(),
    });
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to convert to JS value: {}", e)))
}

/// Get required signatures for a state transition
#[wasm_bindgen(js_name = getRequiredSignaturesForStateTransition)]
pub fn get_required_signatures_for_state_transition(
    state_transition_bytes: &Uint8Array,
) -> Result<JsValue, JsError> {
    use dpp::state_transition::StateTransition;
    use dpp::serialization::PlatformDeserializable;
    
    let bytes = state_transition_bytes.to_vec();
    let platform_version = platform_version::version::PlatformVersion::latest();
    
    let state_transition = StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
        .map_err(|e| JsError::new(&format!("Invalid state transition: {}", e)))?;
    
    let signatures_required = if state_transition.is_identity_signed() {
        serde_json::json!({
            "identitySignature": true,
            "assetLockProof": false,
        })
    } else {
        serde_json::json!({
            "identitySignature": false,
            "assetLockProof": true,
        })
    };
    
    serde_wasm_bindgen::to_value(&signatures_required)
        .map_err(|e| JsError::new(&format!("Failed to convert to JS value: {}", e)))
}