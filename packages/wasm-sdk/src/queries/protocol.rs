use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ProtocolVersionUpgradeState {
    current_protocol_version: u32,
    next_protocol_version: Option<u32>,
    activation_height: Option<u64>,
    vote_count: Option<u32>,
    threshold_reached: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ProtocolVersionUpgradeVoteStatus {
    pro_tx_hash: String,
    voted: bool,
    vote_choice: Option<bool>, // true = yes, false = no
}

#[wasm_bindgen]
pub async fn get_protocol_version_upgrade_state(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    // For now, we'll return the current protocol version
    // In the future, this would query the actual upgrade state from the platform
    let current_version = sdk.version();
    
    let state = ProtocolVersionUpgradeState {
        current_protocol_version: current_version,
        next_protocol_version: None,
        activation_height: None,
        vote_count: None,
        threshold_reached: false,
    };
    
    serde_wasm_bindgen::to_value(&state)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_protocol_version_upgrade_vote_status(
    _sdk: &WasmSdk,
    start_pro_tx_hash: &str,
    count: u32,
) -> Result<JsValue, JsError> {
    // For now, return a mock response
    // In the future, this would query actual masternode votes
    let mut votes = Vec::new();
    
    // Create mock vote status entries
    for i in 0..count.min(5) {
        votes.push(ProtocolVersionUpgradeVoteStatus {
            pro_tx_hash: format!("{}{:02}", start_pro_tx_hash, i),
            voted: i % 2 == 0,
            vote_choice: if i % 2 == 0 { Some(true) } else { None },
        });
    }
    
    serde_wasm_bindgen::to_value(&votes)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}