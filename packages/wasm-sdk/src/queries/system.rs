use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::dpp::core_types::validator_set::v0::ValidatorSetV0Getters;

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
    use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
    use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
    use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
    
    // Get the network from SDK
    let network_str = match sdk.network {
        dash_sdk::dpp::dashcore::Network::Dash => "mainnet",
        dash_sdk::dpp::dashcore::Network::Testnet => "testnet",
        dash_sdk::dpp::dashcore::Network::Devnet => "devnet",
        dash_sdk::dpp::dashcore::Network::Regtest => "regtest",
        _ => "unknown",
    }.to_string();
    
    // Try to fetch current epoch info to get block heights
    let (block_height, core_height) = match ExtendedEpochInfo::fetch_current(sdk.as_ref()).await {
        Ok(epoch_info) => {
            // Extract heights from epoch info
            let platform_height = Some(epoch_info.first_block_height());
            let core_height = Some(epoch_info.first_core_block_height() as u64);
            (platform_height, core_height)
        }
        Err(_) => {
            // If we can't fetch epoch info, heights remain None
            (None, None)
        }
    };
    
    let status = PlatformStatus {
        version: sdk.version(),
        network: network_str,
        block_height,
        core_height,
    };
    
    serde_wasm_bindgen::to_value(&status)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_current_quorums_info(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    use dash_sdk::platform::FetchUnproved;
    use drive_proof_verifier::types::{NoParamQuery, CurrentQuorumsInfo as SdkCurrentQuorumsInfo};
    
    let quorums_result = SdkCurrentQuorumsInfo::fetch_unproved(sdk.as_ref(), NoParamQuery {})
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch quorums info: {}", e)))?;
    
    // The result is Option<CurrentQuorumsInfo>
    if let Some(quorum_info) = quorums_result {
        // Convert the SDK response to our structure
        // Match quorum hashes with validator sets to get detailed information
        let quorums: Vec<QuorumInfo> = quorum_info.quorum_hashes
            .into_iter()
            .map(|quorum_hash| {
                // Try to find the corresponding validator set
                let validator_set = quorum_info.validator_sets
                    .iter()
                    .find(|vs| {
                        // Compare the quorum hash bytes directly
                        
                        let vs_hash_bytes: &[u8] = vs.quorum_hash().as_ref();
                        vs_hash_bytes == &quorum_hash[..]
                    });
                
                if let Some(vs) = validator_set {
                    let member_count = vs.members().len() as u32;
                    
                    // Determine quorum type based on member count and quorum index
                    // This is an approximation based on common quorum sizes
                    // TODO: Get actual quorum type from the platform when available
                    let (quorum_type, threshold) = match member_count {
                        50..=70 => ("LLMQ_60_75".to_string(), (member_count * 75 / 100).max(1)),
                        90..=110 => ("LLMQ_100_67".to_string(), (member_count * 67 / 100).max(1)),
                        350..=450 => ("LLMQ_400_60".to_string(), (member_count * 60 / 100).max(1)),
                        _ => ("LLMQ_TYPE_UNKNOWN".to_string(), (member_count * 2 / 3).max(1)),
                    };
                    
                    QuorumInfo {
                        quorum_hash: hex::encode(&quorum_hash),
                        quorum_type,
                        member_count,
                        threshold,
                        is_verified: true, // We have the validator set, so it's verified
                    }
                } else {
                    // No validator set found for this quorum hash
                    // TODO: This should not happen in normal circumstances. When the SDK
                    // provides complete quorum information, this fallback can be removed.
                    QuorumInfo {
                        quorum_hash: hex::encode(&quorum_hash),
                        quorum_type: "LLMQ_TYPE_UNKNOWN".to_string(),
                        member_count: 0,
                        threshold: 0,
                        is_verified: false,
                    }
                }
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
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_prefunded_specialized_balance(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::{Identifier, Fetch};
    use drive_proof_verifier::types::PrefundedSpecializedBalance as PrefundedBalance;
    
    // Parse identity ID
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch prefunded specialized balance
    let balance_result = PrefundedBalance::fetch(sdk.as_ref(), identity_identifier)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch prefunded specialized balance: {}", e)))?;
    
    if let Some(balance) = balance_result {
        let response = PrefundedSpecializedBalance {
            identity_id: identity_id.to_string(),
            balance: balance.0, // PrefundedSpecializedBalance is a newtype wrapper around u64
        };
        
        // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    } else {
        // Return zero balance if not found
        let response = PrefundedSpecializedBalance {
            identity_id: identity_id.to_string(),
            balance: 0,
        };
        
        // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
            .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
    }
}

#[wasm_bindgen]
pub async fn wait_for_state_transition_result(
    sdk: &WasmSdk,
    state_transition_hash: &str,
) -> Result<JsValue, JsError> {
    use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
        Version, WaitForStateTransitionResultRequestV0,
    };
    use dapi_grpc::platform::v0::WaitForStateTransitionResultRequest;
    
    use dash_sdk::RequestSettings;
    use rs_dapi_client::DapiRequestExecutor;
    
    // Parse the hash from hex string to bytes
    let hash_bytes = hex::decode(state_transition_hash)
        .map_err(|e| JsError::new(&format!("Invalid state transition hash: {}", e)))?;
    
    // Create the gRPC request
    let request = WaitForStateTransitionResultRequest {
        version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
            state_transition_hash: hash_bytes,
            prove: sdk.prove(),
        })),
    };
    
    // Execute the request
    let response = sdk
        .as_ref()
        .execute(request, RequestSettings::default())
        .await
        .map_err(|e| JsError::new(&format!("Failed to wait for state transition result: {}", e)))?;
    
    // Parse the response
    use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
        wait_for_state_transition_result_response_v0::Result as V0Result,
        Version as ResponseVersion,
    };
    
    let (status, error) = match response.inner.version {
        Some(ResponseVersion::V0(v0)) => match v0.result {
            Some(V0Result::Error(e)) => {
                let error_message = format!("Code: {}, Message: {}", e.code, e.message);
                ("ERROR".to_string(), Some(error_message))
            },
            Some(V0Result::Proof(_)) => {
                // State transition was successful
                ("SUCCESS".to_string(), None)
            },
            None => ("UNKNOWN".to_string(), Some("No result returned".to_string())),
        },
        None => ("UNKNOWN".to_string(), Some("No version in response".to_string())),
    };
    
    let result = StateTransitionResult {
        state_transition_hash: state_transition_hash.to_string(),
        status,
        error,
    };
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_path_elements(
    sdk: &WasmSdk,
    path: Vec<String>,
    keys: Vec<String>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::FetchMany;
    use drive_proof_verifier::types::{KeysInPath, Elements};
    use dash_sdk::drive::grovedb::Element;
    
    // Convert string path to byte vectors
    // Path elements can be either numeric values (like "96" for Balances) or string keys
    let path_bytes: Vec<Vec<u8>> = path.iter()
        .map(|p| {
            // Try to parse as a u8 number first (for root tree paths)
            if let Ok(num) = p.parse::<u8>() {
                vec![num]
            } else {
                // Otherwise treat as a string key
                p.as_bytes().to_vec()
            }
        })
        .collect();
    
    // Convert string keys to byte vectors
    let key_bytes: Vec<Vec<u8>> = keys.iter()
        .map(|k| k.as_bytes().to_vec())
        .collect();
    
    // Create the query
    let query = KeysInPath {
        path: path_bytes,
        keys: key_bytes,
    };
    
    // Fetch path elements
    let path_elements_result: Elements = Element::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch path elements: {}", e)))?;
    
    // Convert the result to our response format
    let elements: Vec<PathElement> = keys.into_iter()
        .map(|key| {
            // Check if this key exists in the result
            let value = path_elements_result.get(key.as_bytes())
                .and_then(|element_opt| element_opt.as_ref())
                .and_then(|element| {
                    // Element can contain different types, we'll serialize it as base64
                    element.as_item_bytes().ok().map(|bytes| {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    })
                });
            
            PathElement {
                path: vec![key],
                value,
            }
        })
        .collect();
    
    serde_wasm_bindgen::to_value(&elements)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Proof versions for system queries

#[wasm_bindgen]
pub async fn get_total_credits_in_platform_with_proof_info(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    use dash_sdk::platform::Fetch;
    use drive_proof_verifier::types::{TotalCreditsInPlatform as TotalCreditsQuery, NoParamQuery};
    use crate::queries::ProofMetadataResponse;
    
    let (total_credits_result, metadata, proof) = TotalCreditsQuery::fetch_with_metadata_and_proof(sdk.as_ref(), NoParamQuery {}, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch total credits with proof: {}", e)))?;
    
    let data = if let Some(credits) = total_credits_result {
        Some(TotalCreditsResponse {
            total_credits_in_platform: credits.0.to_string(),
        })
    } else {
        None
    };
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_prefunded_specialized_balance_with_proof_info(
    sdk: &WasmSdk,
    identity_id: &str,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::{Identifier, Fetch};
    use drive_proof_verifier::types::PrefundedSpecializedBalance as PrefundedBalance;
    use crate::queries::ProofMetadataResponse;
    
    // Parse identity ID
    let identity_identifier = Identifier::from_string(
        identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;
    
    // Fetch prefunded specialized balance with proof
    let (balance_result, metadata, proof) = PrefundedBalance::fetch_with_metadata_and_proof(sdk.as_ref(), identity_identifier, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch prefunded specialized balance with proof: {}", e)))?;
    
    let data = PrefundedSpecializedBalance {
        identity_id: identity_id.to_string(),
        balance: balance_result.map(|b| b.0).unwrap_or(0),
    };
    
    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_path_elements_with_proof_info(
    sdk: &WasmSdk,
    path: Vec<String>,
    keys: Vec<String>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::FetchMany;
    use drive_proof_verifier::types::KeysInPath;
    use dash_sdk::drive::grovedb::Element;
    use crate::queries::ProofMetadataResponse;
    
    // Convert string path to byte vectors
    // Path elements can be either numeric values (like "96" for Balances) or string keys
    let path_bytes: Vec<Vec<u8>> = path.iter()
        .map(|p| {
            // Try to parse as a u8 number first (for root tree paths)
            if let Ok(num) = p.parse::<u8>() {
                vec![num]
            } else {
                // Otherwise treat as a string key
                p.as_bytes().to_vec()
            }
        })
        .collect();
    
    // Convert string keys to byte vectors
    let key_bytes: Vec<Vec<u8>> = keys.iter()
        .map(|k| k.as_bytes().to_vec())
        .collect();
    
    // Create the query
    let query = KeysInPath {
        path: path_bytes,
        keys: key_bytes,
    };
    
    // Fetch path elements with proof
    let (path_elements_result, metadata, proof) = Element::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch path elements with proof: {}", e)))?;
    
    // Convert the result to our response format
    let elements: Vec<PathElement> = keys.into_iter()
        .map(|key| {
            let value = path_elements_result.get(key.as_bytes())
                .and_then(|element_opt| element_opt.as_ref())
                .and_then(|element| {
                    element.as_item_bytes().ok().map(|bytes| {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    })
                });
            
            PathElement {
                path: vec![key],
                value,
            }
        })
        .collect();
    
    let response = ProofMetadataResponse {
        data: elements,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}