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
    version: u32,
}

#[wasm_bindgen]
pub async fn get_protocol_version_upgrade_state(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    use dash_sdk::platform::FetchMany;
    use drive_proof_verifier::types::ProtocolVersionVoteCount;
    
    let upgrade_result: drive_proof_verifier::types::ProtocolVersionUpgrades = ProtocolVersionVoteCount::fetch_many(sdk.as_ref(), ())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch protocol version upgrade state: {}", e)))?;
    
    // Get the current protocol version from the SDK
    let current_version = sdk.version();
    
    // Find the next version with votes
    let mut next_version = None;
    let mut activation_height = None;
    let mut vote_count = None;
    let mut threshold_reached = false;
    
    // The result is an IndexMap<u32, Option<u64>> where u32 is version and Option<u64> is activation height
    for (version, height_opt) in upgrade_result.iter() {
        if *version > current_version {
            next_version = Some(*version);
            activation_height = *height_opt;
            // TODO: Get actual vote count and threshold from platform
            vote_count = None;
            threshold_reached = height_opt.is_some();
            break;
        }
    }
    
    let state = ProtocolVersionUpgradeState {
        current_protocol_version: current_version,
        next_protocol_version: next_version,
        activation_height,
        vote_count,
        threshold_reached,
    };
    
    serde_wasm_bindgen::to_value(&state)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_protocol_version_upgrade_vote_status(
    sdk: &WasmSdk,
    start_pro_tx_hash: &str,
    count: u32,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::version_votes::MasternodeProtocolVoteEx;
    use drive_proof_verifier::types::MasternodeProtocolVote;
    use dash_sdk::dpp::dashcore::ProTxHash;
    use std::str::FromStr;
    
    // Parse the ProTxHash
    let start_hash = if start_pro_tx_hash.is_empty() {
        None
    } else {
        Some(ProTxHash::from_str(start_pro_tx_hash)
            .map_err(|e| JsError::new(&format!("Invalid ProTxHash: {}", e)))?)
    };
    
    let votes_result = MasternodeProtocolVote::fetch_votes(sdk.as_ref(), start_hash, Some(count))
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch protocol version votes: {}", e)))?;
    
    // Convert to our response format
    let votes: Vec<ProtocolVersionUpgradeVoteStatus> = votes_result
        .into_iter()
        .filter_map(|(pro_tx_hash, vote_opt)| {
            // vote_opt is Option<MasternodeProtocolVote>
            vote_opt.map(|vote| ProtocolVersionUpgradeVoteStatus {
                pro_tx_hash: pro_tx_hash.to_string(),
                version: vote.voted_version,
            })
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&votes)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}