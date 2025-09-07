use crate::sdk::WasmSdk;
use crate::queries::ProofMetadataResponse;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use serde::{Serialize, Deserialize};
use dash_sdk::platform::{FetchMany, LimitQuery};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dash_sdk::dpp::dashcore::hashes::Hash;
use dash_sdk::dpp::dashcore::ProTxHash;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EpochInfo {
    index: u16,
    first_core_block_height: u32,
    first_block_height: u64,
    start_time: u64,
    fee_multiplier: f64,
    protocol_version: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EvonodesProposedBlocksResponse {
    evonodes_proposed_block_counts: BTreeMap<String, u64>,
}

impl From<ExtendedEpochInfo> for EpochInfo {
    fn from(epoch: ExtendedEpochInfo) -> Self {
        Self {
            index: epoch.index(),
            first_core_block_height: epoch.first_core_block_height(),
            first_block_height: epoch.first_block_height(),
            start_time: epoch.first_block_time(),
            fee_multiplier: epoch.fee_multiplier_permille() as f64 / 1000.0,
            protocol_version: epoch.protocol_version(),
        }
    }
}

#[wasm_bindgen]
pub async fn get_epochs_info(
    sdk: &WasmSdk,
    start_epoch: Option<u16>,
    count: Option<u32>,
    ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::epoch::EpochQuery;
    
    let query = LimitQuery {
        query: EpochQuery {
            start: start_epoch,
            ascending: ascending.unwrap_or(true),
        },
        limit: count,
        start_info: None,
    };
    
    let epochs_result: drive_proof_verifier::types::ExtendedEpochInfos = ExtendedEpochInfo::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch epochs info: {}", e)))?;
    
    // Convert to our response format
    let epochs: Vec<EpochInfo> = epochs_result
        .into_iter()
        .filter_map(|(_, epoch_opt)| epoch_opt.map(Into::into))
        .collect();
    
    serde_wasm_bindgen::to_value(&epochs)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_finalized_epoch_infos(
    sdk: &WasmSdk,
    start_epoch: Option<u16>,
    count: Option<u32>,
    ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::finalized_epoch::FinalizedEpochQuery;
    
    if start_epoch.is_none() {
        return Err(JsError::new("start_epoch is required for finalized epoch queries"));
    }
    
    let start = start_epoch.unwrap();
    let is_ascending = ascending.unwrap_or(true);
    let limit = count.unwrap_or(100);
    
    // Ensure limit is at least 1 to avoid underflow
    let limit = limit.max(1);
    
    // Calculate end epoch based on direction and limit
    let end_epoch = if is_ascending {
        start.saturating_add((limit - 1) as u16)
    } else {
        start.saturating_sub((limit - 1) as u16)
    };
    
    let query = if is_ascending {
        FinalizedEpochQuery {
            start_epoch_index: start,
            start_epoch_index_included: true,
            end_epoch_index: end_epoch,
            end_epoch_index_included: true,
        }
    } else {
        FinalizedEpochQuery {
            start_epoch_index: end_epoch,
            start_epoch_index_included: true,
            end_epoch_index: start,
            end_epoch_index_included: true,
        }
    };
    
    let epochs_result: drive_proof_verifier::types::FinalizedEpochInfos = dash_sdk::dpp::block::finalized_epoch_info::FinalizedEpochInfo::fetch_many(sdk.as_ref(), query)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch finalized epochs info: {}", e)))?;
    
    // Convert to our response format and sort by epoch index
    let mut epochs: Vec<EpochInfo> = epochs_result
        .into_iter()
        .filter_map(|(epoch_index, epoch_opt)| {
            epoch_opt.map(|epoch| {
                use dash_sdk::dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
                EpochInfo {
                    index: epoch_index as u16,
                    first_core_block_height: epoch.first_core_block_height(),
                    first_block_height: epoch.first_block_height(),
                    start_time: epoch.first_block_time(),
                    fee_multiplier: epoch.fee_multiplier_permille() as f64 / 1000.0,
                    protocol_version: epoch.protocol_version(),
                }
            })
        })
        .collect();
    
    // Sort based on ascending flag
    epochs.sort_by(|a, b| {
        if is_ascending {
            a.index.cmp(&b.index)
        } else {
            b.index.cmp(&a.index)
        }
    });
    
    serde_wasm_bindgen::to_value(&epochs)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ProposerBlockCount {
    proposer_pro_tx_hash: String,
    count: u64,
}

#[wasm_bindgen]
pub async fn get_evonodes_proposed_epoch_blocks_by_ids(
    sdk: &WasmSdk,
    epoch: u16,
    ids: Vec<String>,
) -> Result<JsValue, JsError> {
    use drive_proof_verifier::types::ProposerBlockCountById;
    
    // Parse the ProTxHash strings
    let pro_tx_hashes: Vec<ProTxHash> = ids
        .into_iter()
        .map(|hash_str| {
            ProTxHash::from_str(&hash_str)
                .map_err(|e| JsError::new(&format!("Invalid ProTxHash '{}': {}", hash_str, e)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    
    // Use FetchMany to get block counts for specific IDs
    let counts = ProposerBlockCountById::fetch_many(sdk.as_ref(), (epoch, pro_tx_hashes))
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch evonode proposed blocks: {}", e)))?;
    
    // Convert to response format
    let mut evonodes_proposed_block_counts = BTreeMap::new();
    for (identifier, count) in counts.0 {
        // Convert Identifier to ProTxHash for consistent output format
        let bytes = identifier.to_buffer();
        if bytes.len() == 32 {
            let hash_array: [u8; 32] = bytes.try_into().unwrap();
            let pro_tx_hash = ProTxHash::from_byte_array(hash_array);
            evonodes_proposed_block_counts.insert(pro_tx_hash.to_string(), count);
        }
    }
    
    let response = EvonodesProposedBlocksResponse {
        evonodes_proposed_block_counts,
    };
    
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_evonodes_proposed_epoch_blocks_by_range(
    sdk: &WasmSdk,
    epoch: u16,
    limit: Option<u32>,
    start_after: Option<String>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::proposed_blocks::ProposedBlockCountEx;
    use drive_proof_verifier::types::ProposerBlockCounts;
        use dash_sdk::platform::QueryStartInfo;
    
    // Parse start_after if provided
    let start_info = if let Some(start) = start_after {
        let pro_tx_hash = ProTxHash::from_str(&start)
            .map_err(|e| JsError::new(&format!("Invalid start_after ProTxHash: {}", e)))?;
        Some(QueryStartInfo {
            start_key: pro_tx_hash.to_byte_array().to_vec(),
            start_included: false,
        })
    } else {
        None
    };
    
    let counts_result = ProposerBlockCounts::fetch_proposed_blocks_by_range(
        sdk.as_ref(),
        Some(epoch),
        limit,
        start_info,
    )
    .await
    .map_err(|e| JsError::new(&format!("Failed to fetch evonode proposed blocks by range: {}", e)))?;
    
    // Convert to response format
    let mut responses: Vec<ProposerBlockCount> = counts_result.0
        .into_iter()
        .map(|(identifier, count)| {
            // Convert Identifier back to ProTxHash
            let bytes = identifier.to_buffer();
            let hash = dash_sdk::dpp::dashcore::hashes::sha256d::Hash::from_slice(&bytes).unwrap();
            let pro_tx_hash = ProTxHash::from_raw_hash(hash);
            ProposerBlockCount {
                proposer_pro_tx_hash: pro_tx_hash.to_string(),
                count,
            }
        })
        .collect();
    
    // Sort based on order_ascending (default is true)
    let ascending = order_ascending.unwrap_or(true);
    responses.sort_by(|a, b| {
        if ascending {
            a.proposer_pro_tx_hash.cmp(&b.proposer_pro_tx_hash)
        } else {
            b.proposer_pro_tx_hash.cmp(&a.proposer_pro_tx_hash)
        }
    });
    
    serde_wasm_bindgen::to_value(&responses)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_current_epoch(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    let epoch = ExtendedEpochInfo::fetch_current(sdk.as_ref())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch current epoch: {}", e)))?;
    
    let epoch_info = EpochInfo::from(epoch);
    
    serde_wasm_bindgen::to_value(&epoch_info)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_epochs_info_with_proof_info(
    sdk: &WasmSdk,
    start_epoch: Option<u16>,
    count: Option<u32>,
    ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::epoch::EpochQuery;
    
    let query = LimitQuery {
        query: EpochQuery {
            start: start_epoch,
            ascending: ascending.unwrap_or(true),
        },
        limit: count,
        start_info: None,
    };
    
    let (epochs_result, metadata, proof) = ExtendedEpochInfo::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch epochs info with proof: {}", e)))?;
    
    // Convert to our response format
    let epochs: Vec<EpochInfo> = epochs_result
        .into_iter()
        .filter_map(|(_, epoch_opt)| epoch_opt.map(Into::into))
        .collect();
    
    let response = ProofMetadataResponse {
        data: epochs,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_current_epoch_with_proof_info(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    let (epoch, metadata, proof) = ExtendedEpochInfo::fetch_current_with_metadata_and_proof(sdk.as_ref())
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch current epoch with proof: {}", e)))?;
    
    let epoch_info = EpochInfo::from(epoch);
    
    let response = ProofMetadataResponse {
        data: epoch_info,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

// Additional proof info versions for epoch queries

#[wasm_bindgen]
pub async fn get_finalized_epoch_infos_with_proof_info(
    sdk: &WasmSdk,
    start_epoch: Option<u16>,
    count: Option<u32>,
    ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    use dash_sdk::platform::types::finalized_epoch::FinalizedEpochQuery;
    
    
    if start_epoch.is_none() {
        return Err(JsError::new("start_epoch is required for finalized epoch queries"));
    }
    
    let start = start_epoch.unwrap();
    let is_ascending = ascending.unwrap_or(true);
    let limit = count.unwrap_or(100);
    
    // Ensure limit is at least 1 to avoid underflow
    let limit = limit.max(1);
    
    // Calculate end epoch based on direction and limit
    let end_epoch = if is_ascending {
        start.saturating_add((limit - 1) as u16)
    } else {
        start.saturating_sub((limit - 1) as u16)
    };
    
    let query = if is_ascending {
        FinalizedEpochQuery {
            start_epoch_index: start,
            start_epoch_index_included: true,
            end_epoch_index: end_epoch,
            end_epoch_index_included: true,
        }
    } else {
        FinalizedEpochQuery {
            start_epoch_index: end_epoch,
            start_epoch_index_included: true,
            end_epoch_index: start,
            end_epoch_index_included: true,
        }
    };
    
    let (epochs_result, metadata, proof) = dash_sdk::dpp::block::finalized_epoch_info::FinalizedEpochInfo::fetch_many_with_metadata_and_proof(sdk.as_ref(), query, None)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch finalized epochs info with proof: {}", e)))?;
    
    // Convert to our response format and sort by epoch index
    let mut epochs: Vec<EpochInfo> = epochs_result
        .into_iter()
        .filter_map(|(epoch_index, epoch_opt)| {
            epoch_opt.map(|epoch| {
                use dash_sdk::dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
                EpochInfo {
                    index: epoch_index as u16,
                    first_core_block_height: epoch.first_core_block_height(),
                    first_block_height: epoch.first_block_height(),
                    start_time: epoch.first_block_time(),
                    fee_multiplier: epoch.fee_multiplier_permille() as f64 / 1000.0,
                    protocol_version: epoch.protocol_version(),
                }
            })
        })
        .collect();
    
    // Sort based on ascending flag
    epochs.sort_by(|a, b| {
        if is_ascending {
            a.index.cmp(&b.index)
        } else {
            b.index.cmp(&a.index)
        }
    });
    
    let response = ProofMetadataResponse {
        data: epochs,
        metadata: metadata.into(),
        proof: proof.into(),
    };
    
    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response.serialize(&serializer)
        .map_err(|e| JsError::new(&format!("Failed to serialize response: {}", e)))
}

#[wasm_bindgen]
pub async fn get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info(
    sdk: &WasmSdk,
    epoch: u16,
    pro_tx_hashes: Vec<String>,
) -> Result<JsValue, JsError> {
    // TODO: Implement once SDK Query trait is implemented for ProposerBlockCountById
    // Currently not supported due to query format issues
    let _ = (sdk, epoch, pro_tx_hashes); // Parameters will be used when implemented
    Err(JsError::new("get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info is not yet implemented"))
}

#[wasm_bindgen]
pub async fn get_evonodes_proposed_epoch_blocks_by_range_with_proof_info(
    sdk: &WasmSdk,
    epoch: u16,
    limit: Option<u32>,
    start_after: Option<String>,
    order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    // TODO: Implement once SDK Query trait is implemented for ProposerBlockCountByRange
    // Currently not supported due to query format issues
    let _ = (sdk, epoch, limit, start_after, order_ascending); // Parameters will be used when implemented
    Err(JsError::new("get_evonodes_proposed_epoch_blocks_by_range_with_proof_info is not yet implemented"))
}