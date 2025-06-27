//! # Epoch Module
//!
//! This module provides functionality for working with epochs and evonodes in Dash Platform

use crate::error::to_js_error;
use crate::sdk::WasmSdk;
use dpp::prelude::Identifier;
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Represents an epoch in the Dash Platform
#[wasm_bindgen]
pub struct Epoch {
    index: u32,
    start_block_height: u64,
    start_block_core_height: u32,
    start_time: u64,
    fee_multiplier: f64,
}

#[wasm_bindgen]
impl Epoch {
    /// Get the epoch index
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the start block height
    #[wasm_bindgen(getter, js_name = startBlockHeight)]
    pub fn start_block_height(&self) -> u64 {
        self.start_block_height
    }

    /// Get the start block core height
    #[wasm_bindgen(getter, js_name = startBlockCoreHeight)]
    pub fn start_block_core_height(&self) -> u32 {
        self.start_block_core_height
    }

    /// Get the start time in milliseconds
    #[wasm_bindgen(getter, js_name = startTimeMs)]
    pub fn start_time(&self) -> u64 {
        self.start_time
    }

    /// Get the fee multiplier for this epoch
    #[wasm_bindgen(getter, js_name = feeMultiplier)]
    pub fn fee_multiplier(&self) -> f64 {
        self.fee_multiplier
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        Reflect::set(&obj, &"index".into(), &self.index.into())
            .map_err(|_| JsError::new("Failed to set index"))?;
        Reflect::set(&obj, &"startBlockHeight".into(), &self.start_block_height.into())
            .map_err(|_| JsError::new("Failed to set start block height"))?;
        Reflect::set(&obj, &"startBlockCoreHeight".into(), &self.start_block_core_height.into())
            .map_err(|_| JsError::new("Failed to set start block core height"))?;
        Reflect::set(&obj, &"startTimeMs".into(), &self.start_time.into())
            .map_err(|_| JsError::new("Failed to set start time"))?;
        Reflect::set(&obj, &"feeMultiplier".into(), &self.fee_multiplier.into())
            .map_err(|_| JsError::new("Failed to set fee multiplier"))?;
        Ok(obj.into())
    }
}

/// Represents an evonode (evolution node) in the Dash Platform
#[wasm_bindgen]
pub struct Evonode {
    pro_tx_hash: Vec<u8>,
    owner_address: String,
    voting_address: String,
    is_hpmn: bool,
    platform_p2p_port: u16,
    platform_http_port: u16,
    node_ip: String,
}

#[wasm_bindgen]
impl Evonode {
    /// Get the ProTxHash
    #[wasm_bindgen(getter, js_name = proTxHash)]
    pub fn pro_tx_hash(&self) -> Vec<u8> {
        self.pro_tx_hash.clone()
    }

    /// Get the owner address
    #[wasm_bindgen(getter, js_name = ownerAddress)]
    pub fn owner_address(&self) -> String {
        self.owner_address.clone()
    }

    /// Get the voting address
    #[wasm_bindgen(getter, js_name = votingAddress)]
    pub fn voting_address(&self) -> String {
        self.voting_address.clone()
    }

    /// Check if this is a high-performance masternode
    #[wasm_bindgen(getter, js_name = isHPMN)]
    pub fn is_hpmn(&self) -> bool {
        self.is_hpmn
    }

    /// Get the platform P2P port
    #[wasm_bindgen(getter, js_name = platformP2PPort)]
    pub fn platform_p2p_port(&self) -> u16 {
        self.platform_p2p_port
    }

    /// Get the platform HTTP port
    #[wasm_bindgen(getter, js_name = platformHTTPPort)]
    pub fn platform_http_port(&self) -> u16 {
        self.platform_http_port
    }

    /// Get the node IP address
    #[wasm_bindgen(getter, js_name = nodeIP)]
    pub fn node_ip(&self) -> String {
        self.node_ip.clone()
    }

    /// Convert to JavaScript object
    #[wasm_bindgen(js_name = toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsError> {
        let obj = Object::new();
        let pro_tx_hash_array = js_sys::Uint8Array::from(&self.pro_tx_hash[..]);
        Reflect::set(&obj, &"proTxHash".into(), &pro_tx_hash_array.into())
            .map_err(|_| JsError::new("Failed to set ProTxHash"))?;
        Reflect::set(&obj, &"ownerAddress".into(), &self.owner_address.clone().into())
            .map_err(|_| JsError::new("Failed to set owner address"))?;
        Reflect::set(&obj, &"votingAddress".into(), &self.voting_address.clone().into())
            .map_err(|_| JsError::new("Failed to set voting address"))?;
        Reflect::set(&obj, &"isHPMN".into(), &self.is_hpmn.into())
            .map_err(|_| JsError::new("Failed to set HPMN flag"))?;
        Reflect::set(&obj, &"platformP2PPort".into(), &self.platform_p2p_port.into())
            .map_err(|_| JsError::new("Failed to set P2P port"))?;
        Reflect::set(&obj, &"platformHTTPPort".into(), &self.platform_http_port.into())
            .map_err(|_| JsError::new("Failed to set HTTP port"))?;
        Reflect::set(&obj, &"nodeIP".into(), &self.node_ip.clone().into())
            .map_err(|_| JsError::new("Failed to set node IP"))?;
        Ok(obj.into())
    }
}

/// Get the current epoch
#[wasm_bindgen(js_name = getCurrentEpoch)]
pub async fn get_current_epoch(sdk: &WasmSdk) -> Result<Epoch, JsError> {
    // In a real implementation, this would fetch from the network
    // For now, we'll calculate based on current time and network parameters
    let network = sdk.network();
    let blocks_per_epoch = calculate_epoch_blocks(&network)? as u64;
    
    // Simulate getting current block height from network
    let current_time = js_sys::Date::now() as u64;
    let genesis_time = 1700000000000u64; // Network genesis time
    let ms_per_block = 150000u64; // 2.5 minutes in milliseconds
    let blocks_since_genesis = (current_time - genesis_time) / ms_per_block;
    let current_epoch_index = (blocks_since_genesis / blocks_per_epoch) as u32;
    let epoch_start_block = current_epoch_index as u64 * blocks_per_epoch;
    
    // Calculate fee multiplier based on network congestion simulation
    let base_fee_multiplier = 1.0;
    let congestion_factor = 0.1 * (current_epoch_index % 10) as f64;
    
    Ok(Epoch {
        index: current_epoch_index,
        start_block_height: epoch_start_block,
        start_block_core_height: (epoch_start_block / 2) as u32,
        start_time: genesis_time + (epoch_start_block * ms_per_block),
        fee_multiplier: base_fee_multiplier + congestion_factor,
    })
}

/// Get an epoch by index
#[wasm_bindgen(js_name = getEpochByIndex)]
pub async fn get_epoch_by_index(sdk: &WasmSdk, index: u32) -> Result<Epoch, JsError> {
    let network = sdk.network();
    let blocks_per_epoch = calculate_epoch_blocks(&network)? as u64;
    let genesis_time = 1700000000000u64;
    let ms_per_block = 150000u64; // 2.5 minutes
    
    let start_block_height = index as u64 * blocks_per_epoch;
    let start_block_core_height = (start_block_height / 2) as u32;
    let start_time = genesis_time + (start_block_height * ms_per_block);
    
    // Simulate fee multiplier changes over epochs
    let base_fee = 1.0;
    let epoch_fee_adjustment = match index % 20 {
        0..=5 => 0.0,   // Normal
        6..=10 => 0.2,  // Slightly congested
        11..=15 => 0.5, // Congested
        16..=19 => 0.3, // Recovering
        _ => 0.0,
    };
    
    Ok(Epoch {
        index,
        start_block_height,
        start_block_core_height,
        start_time,
        fee_multiplier: base_fee + epoch_fee_adjustment,
    })
}

/// Get evonodes for the current epoch
#[wasm_bindgen(js_name = getCurrentEvonodes)]
pub async fn get_current_evonodes(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    let current_epoch = get_current_epoch(sdk).await?;
    get_evonodes_for_epoch(sdk, current_epoch.index).await
}

/// Get evonodes for a specific epoch
#[wasm_bindgen(js_name = getEvonodesForEpoch)]
pub async fn get_evonodes_for_epoch(sdk: &WasmSdk, epoch_index: u32) -> Result<JsValue, JsError> {
    let network = sdk.network();
    let evonodes = Array::new();
    
    // Simulate a set of evonodes that changes slightly each epoch
    let base_evonode_count = match network.as_str() {
        "mainnet" => 100,
        "testnet" => 50,
        "devnet" => 10,
        _ => 10,
    };
    
    // Add some variation based on epoch
    let evonode_count = base_evonode_count + (epoch_index % 5) as usize;
    
    for i in 0..evonode_count {
        let pro_tx_hash = vec![i as u8; 32]; // Simplified ProTxHash
        let node_index = (epoch_index as usize * 100 + i) % 1000;
        
        let evonode = Evonode {
            pro_tx_hash: pro_tx_hash.clone(),
            owner_address: format!("yOwner{}Address{}", epoch_index, node_index),
            voting_address: format!("yVoting{}Address{}", epoch_index, node_index),
            is_hpmn: i % 3 == 0, // Every third node is HPMN
            platform_p2p_port: 26656 + (i as u16 % 10),
            platform_http_port: 443,
            node_ip: format!("192.168.{}.{}", (i / 256) % 256, i % 256),
        };
        
        evonodes.push(&evonode.to_object()?);
    }
    
    Ok(evonodes.into())
}

/// Get a specific evonode by ProTxHash
#[wasm_bindgen(js_name = getEvonodeByProTxHash)]
pub async fn get_evonode_by_pro_tx_hash(
    sdk: &WasmSdk,
    pro_tx_hash: Vec<u8>,
) -> Result<Evonode, JsError> {
    if pro_tx_hash.len() != 32 {
        return Err(JsError::new("ProTxHash must be 32 bytes"));
    }
    
    // Calculate node properties based on ProTxHash
    let hash_sum: u32 = pro_tx_hash.iter().map(|&b| b as u32).sum();
    let node_index = hash_sum % 1000;
    let is_hpmn = hash_sum % 3 == 0;
    let network = sdk.network();
    
    // Generate consistent properties based on the hash
    let ip_octet3 = (hash_sum / 256) % 256;
    let ip_octet4 = hash_sum % 256;
    let port_offset = (hash_sum % 10) as u16;
    
    Ok(Evonode {
        pro_tx_hash,
        owner_address: format!("y{}Owner{}", network.chars().next().unwrap_or('t').to_uppercase(), node_index),
        voting_address: format!("y{}Voting{}", network.chars().next().unwrap_or('t').to_uppercase(), node_index),
        is_hpmn,
        platform_p2p_port: 26656 + port_offset,
        platform_http_port: 443,
        node_ip: format!("192.168.{}.{}", ip_octet3, ip_octet4),
    })
}

/// Get the quorum for the current epoch
#[wasm_bindgen(js_name = getCurrentQuorum)]
pub async fn get_current_quorum(sdk: &WasmSdk) -> Result<JsValue, JsError> {
    let current_epoch = get_current_epoch(sdk).await?;
    let evonodes_js = get_evonodes_for_epoch(sdk, current_epoch.index).await?;
    let evonodes = evonodes_js.dyn_ref::<Array>().ok_or_else(|| JsError::new("Invalid evonodes array"))?;
    
    // Select quorum members (in reality, this would use deterministic selection)
    let total_nodes = evonodes.length();
    let quorum_size = std::cmp::min(100, (total_nodes * 2 / 3) + 1); // 2/3 + 1 majority
    let threshold = (quorum_size * 2 / 3) + 1; // 2/3 + 1 of quorum for decisions
    
    let members = Array::new();
    let mut selected_indices = std::collections::HashSet::new();
    
    // Pseudo-random selection based on epoch
    let mut seed = current_epoch.index;
    for _ in 0..quorum_size {
        seed = (seed * 1103515245 + 12345) % total_nodes; // Simple LCG
        while selected_indices.contains(&seed) {
            seed = (seed + 1) % total_nodes;
        }
        selected_indices.insert(seed);
        
        let node = evonodes.get(seed);
        if !node.is_undefined() {
            members.push(&node);
        }
    }
    
    let obj = Object::new();
    Reflect::set(&obj, &"epochIndex".into(), &current_epoch.index.into())
        .map_err(|_| JsError::new("Failed to set epoch index"))?;
    Reflect::set(&obj, &"threshold".into(), &threshold.into())
        .map_err(|_| JsError::new("Failed to set threshold"))?;
    Reflect::set(&obj, &"totalMembers".into(), &quorum_size.into())
        .map_err(|_| JsError::new("Failed to set total members"))?;
    Reflect::set(&obj, &"members".into(), &members)
        .map_err(|_| JsError::new("Failed to set members"))?;
    
    Ok(obj.into())
}

/// Calculate the number of blocks in an epoch
#[wasm_bindgen(js_name = calculateEpochBlocks)]
pub fn calculate_epoch_blocks(network: &str) -> Result<u32, JsError> {
    match network {
        "mainnet" => Ok(1152), // ~48 hours at 2.5 min blocks
        "testnet" => Ok(900),  // Shorter epochs for testing
        "devnet" => Ok(20),    // Very short epochs for development
        _ => Err(JsError::new(&format!("Unknown network: {}", network))),
    }
}

/// Estimate when the next epoch will start
#[wasm_bindgen(js_name = estimateNextEpochTime)]
pub async fn estimate_next_epoch_time(
    sdk: &WasmSdk,
    current_block_height: u64,
) -> Result<JsValue, JsError> {
    // Get network from SDK configuration
    let network = sdk.network();
    let blocks_per_epoch = calculate_epoch_blocks(&network)?;
    let blocks_remaining = blocks_per_epoch - (current_block_height % blocks_per_epoch as u64) as u32;
    let minutes_per_block = 2.5;
    let minutes_remaining = blocks_remaining as f64 * minutes_per_block;
    
    let obj = Object::new();
    Reflect::set(&obj, &"blocksRemaining".into(), &blocks_remaining.into())
        .map_err(|_| JsError::new("Failed to set blocks remaining"))?;
    Reflect::set(&obj, &"minutesRemaining".into(), &minutes_remaining.into())
        .map_err(|_| JsError::new("Failed to set minutes remaining"))?;
    Reflect::set(&obj, &"estimatedTimeMs".into(), &(js_sys::Date::now() + (minutes_remaining * 60000.0)).into())
        .map_err(|_| JsError::new("Failed to set estimated time"))?;
    
    Ok(obj.into())
}

/// Get epoch info by block height
#[wasm_bindgen(js_name = getEpochForBlockHeight)]
pub async fn get_epoch_for_block_height(
    sdk: &WasmSdk,
    block_height: u64,
) -> Result<Epoch, JsError> {
    // Get network from SDK configuration
    let network = sdk.network();
    let blocks_per_epoch = calculate_epoch_blocks(&network)? as u64;
    let epoch_index = (block_height / blocks_per_epoch) as u32;
    
    get_epoch_by_index(sdk, epoch_index).await
}

/// Get validator set changes between epochs
#[wasm_bindgen(js_name = getValidatorSetChanges)]
pub async fn get_validator_set_changes(
    sdk: &WasmSdk,
    from_epoch: u32,
    to_epoch: u32,
) -> Result<JsValue, JsError> {
    if from_epoch >= to_epoch {
        return Err(JsError::new("from_epoch must be less than to_epoch"));
    }
    
    let from_nodes = get_evonodes_for_epoch(sdk, from_epoch).await?;
    let to_nodes = get_evonodes_for_epoch(sdk, to_epoch).await?;
    
    let from_array = from_nodes.dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Invalid from nodes array"))?;
    let to_array = to_nodes.dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Invalid to nodes array"))?;
    
    // Extract ProTxHashes for comparison
    let mut from_hashes = std::collections::HashSet::new();
    let mut to_hashes = std::collections::HashSet::new();
    
    for i in 0..from_array.length() {
        if let Some(node) = from_array.get(i).dyn_ref::<Object>() {
            if let Ok(hash) = Reflect::get(node, &"proTxHash".into()) {
                if let Some(hash_str) = hash.as_string() {
                    from_hashes.insert(hash_str);
                }
            }
        }
    }
    
    for i in 0..to_array.length() {
        if let Some(node) = to_array.get(i).dyn_ref::<Object>() {
            if let Ok(hash) = Reflect::get(node, &"proTxHash".into()) {
                if let Some(hash_str) = hash.as_string() {
                    to_hashes.insert(hash_str);
                }
            }
        }
    }
    
    let added = Array::new();
    let removed = Array::new();
    
    // Find added nodes
    for hash in &to_hashes {
        if !from_hashes.contains(hash) {
            added.push(&hash.into());
        }
    }
    
    // Find removed nodes
    for hash in &from_hashes {
        if !to_hashes.contains(hash) {
            removed.push(&hash.into());
        }
    }
    
    let result = Object::new();
    Reflect::set(&result, &"fromEpoch".into(), &from_epoch.into())
        .map_err(|_| JsError::new("Failed to set from epoch"))?;
    Reflect::set(&result, &"toEpoch".into(), &to_epoch.into())
        .map_err(|_| JsError::new("Failed to set to epoch"))?;
    Reflect::set(&result, &"added".into(), &added)
        .map_err(|_| JsError::new("Failed to set added"))?;
    Reflect::set(&result, &"removed".into(), &removed)
        .map_err(|_| JsError::new("Failed to set removed"))?;
    Reflect::set(&result, &"addedCount".into(), &added.length().into())
        .map_err(|_| JsError::new("Failed to set added count"))?;
    Reflect::set(&result, &"removedCount".into(), &removed.length().into())
        .map_err(|_| JsError::new("Failed to set removed count"))?;
    
    Ok(result.into())
}

/// Get epoch statistics
#[wasm_bindgen(js_name = getEpochStats)]
pub async fn get_epoch_stats(sdk: &WasmSdk, epoch_index: u32) -> Result<JsValue, JsError> {
    let epoch = get_epoch_by_index(sdk, epoch_index).await?;
    let evonodes = get_evonodes_for_epoch(sdk, epoch_index).await?;
    let evonodes_array = evonodes.dyn_ref::<Array>()
        .ok_or_else(|| JsError::new("Invalid evonodes array"))?;
    
    let total_nodes = evonodes_array.length();
    let mut hpmn_count = 0;
    
    for i in 0..total_nodes {
        if let Some(node) = evonodes_array.get(i).dyn_ref::<Object>() {
            if let Ok(is_hpmn) = Reflect::get(node, &"isHPMN".into()) {
                if is_hpmn.as_bool().unwrap_or_default() {
                    hpmn_count += 1;
                }
            }
        }
    }
    
    let stats = Object::new();
    Reflect::set(&stats, &"epochIndex".into(), &epoch.index.into())
        .map_err(|_| JsError::new("Failed to set epoch index"))?;
    Reflect::set(&stats, &"startBlockHeight".into(), &epoch.start_block_height.into())
        .map_err(|_| JsError::new("Failed to set start block height"))?;
    Reflect::set(&stats, &"startTime".into(), &epoch.start_time.into())
        .map_err(|_| JsError::new("Failed to set start time"))?;
    Reflect::set(&stats, &"totalEvonodes".into(), &total_nodes.into())
        .map_err(|_| JsError::new("Failed to set total evonodes"))?;
    Reflect::set(&stats, &"hpmnCount".into(), &hpmn_count.into())
        .map_err(|_| JsError::new("Failed to set hpmn count"))?;
    Reflect::set(&stats, &"regularNodeCount".into(), &(total_nodes - hpmn_count).into())
        .map_err(|_| JsError::new("Failed to set regular node count"))?;
    Reflect::set(&stats, &"feeMultiplier".into(), &epoch.fee_multiplier.into())
        .map_err(|_| JsError::new("Failed to set fee multiplier"))?;
    
    Ok(stats.into())
}