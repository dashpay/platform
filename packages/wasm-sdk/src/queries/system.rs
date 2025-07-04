use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PlatformStatus {
    version: u32,
    network: String,
    block_height: Option<u64>,
    core_height: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QuorumInfo {
    quorum_hash: String,
    quorum_type: String,
    member_count: u32,
    threshold: u32,
    is_verified: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CurrentQuorumsInfo {
    quorums: Vec<QuorumInfo>,
    height: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TotalCreditsResponse {
    total_credits_in_platform: String,  // Use String to handle large numbers
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StateTransitionResult {
    state_transition_hash: String,
    status: String,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PrefundedSpecializedBalance {
    identity_id: String,
    balance: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PathElement {
    path: Vec<String>,
    value: Option<String>,
}

#[wasm_bindgen]
pub async fn get_status(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    // TODO: Get actual status from the platform
    let status = PlatformStatus {
        version: sdk.version(),
        network: "testnet".to_string(), // This should come from SDK config
        block_height: None,
        core_height: None,
    };
    
    serde_wasm_bindgen::to_value(&status)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_current_quorums_info(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    use dash_sdk::platform::FetchUnproved;
    use drive_proof_verifier::types::{NoParamQuery, CurrentQuorumsInfo as CurrentQuorumsQuery};
    
    let quorums_result = CurrentQuorumsQuery::fetch_unproved(sdk.as_ref(), NoParamQuery {})
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch quorums info: {}", e)))?;
    
    // The result is Option<CurrentQuorumsInfo>
    if let Some(quorum_info) = quorums_result {
        // Convert the SDK response to our structure
        // For now, we'll use the quorum hashes to create basic info
        let quorums: Vec<QuorumInfo> = quorum_info.quorum_hashes
            .into_iter()
            .map(|quorum_hash| QuorumInfo {
                quorum_hash: hex::encode(&quorum_hash),
                quorum_type: "LLMQ_TYPE_UNKNOWN".to_string(), // Type info not available in this response
                member_count: 0, // Not available
                threshold: 0, // Not available
                is_verified: false, // Not available
            })
            .collect();
        
        let info = CurrentQuorumsInfo {
            quorums,
            height: quorum_info.last_platform_block_height,
        };
        
        serde_wasm_bindgen::to_value(&info)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        // No quorum info available
        let info = CurrentQuorumsInfo {
            quorums: vec![],
            height: 0,
        };
        
        serde_wasm_bindgen::to_value(&info)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    }
}

#[wasm_bindgen]
pub async fn get_total_credits_in_platform(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    use dash_sdk::platform::Fetch;
    use drive_proof_verifier::types::{TotalCreditsInPlatform as TotalCreditsQuery, NoParamQuery};
    
    let total_credits_result = TotalCreditsQuery::fetch(sdk.as_ref(), NoParamQuery {})
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch total credits: {}", e)))?;
    
    // TotalCreditsInPlatform is likely a newtype wrapper around u64
    let credits_value = if let Some(credits) = total_credits_result {
        // Extract the inner value - assuming it has a field or can be dereferenced
        // We'll try to access it as a tuple struct
        credits.0
    } else {
        0
    };
    
    let response = TotalCreditsResponse {
        total_credits_in_platform: credits_value.to_string(),
    };
    
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_prefunded_specialized_balance(
    _sdk: &WasmSdk,
    identity_id: &str,
) -> Result<JsValue, JsError> {
    // TODO: Query actual prefunded balance from the platform
    let response = PrefundedSpecializedBalance {
        identity_id: identity_id.to_string(),
        balance: 0,
    };
    
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn wait_for_state_transition_result(
    _sdk: &WasmSdk,
    state_transition_hash: &str,
) -> Result<JsValue, JsError> {
    // TODO: Implement actual polling for state transition result
    let result = StateTransitionResult {
        state_transition_hash: state_transition_hash.to_string(),
        status: "UNKNOWN".to_string(),
        error: Some("Not implemented - cannot query state transition status".to_string()),
    };
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_path_elements(
    _sdk: &WasmSdk,
    keys: Vec<String>,
) -> Result<JsValue, JsError> {
    // TODO: Query actual path elements from the platform state tree
    let elements: Vec<PathElement> = keys.into_iter().map(|key| {
        PathElement {
            path: vec![key],
            value: None,
        }
    }).collect();
    
    serde_wasm_bindgen::to_value(&elements)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}