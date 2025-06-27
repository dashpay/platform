//! Comprehensive tests for the epoch module

use wasm_bindgen_test::*;
use wasm_sdk::{
    epoch::{
        get_current_epoch, get_epoch_by_index, get_current_evonodes,
        get_evonodes_for_epoch, get_evonode_by_pro_tx_hash, get_current_quorum,
        calculate_epoch_blocks, estimate_next_epoch_time, get_epoch_for_block_height,
        get_validator_set_changes, get_epoch_stats
    },
    sdk::WasmSdk,
    start,
};
use wasm_bindgen::JsValue;
use js_sys::{Array, Object, Reflect};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_epoch_creation() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    // Test getting current epoch
    let current_epoch = get_current_epoch(&sdk).await.expect("Failed to get current epoch");
    
    assert!(current_epoch.index() > 0);
    assert!(current_epoch.start_block_height() > 0);
    assert!(current_epoch.start_time() > 0);
    assert!(current_epoch.fee_multiplier() >= 1.0);
    
    // Test epoch object conversion
    let epoch_obj = current_epoch.to_object().expect("Failed to convert epoch to object");
    assert!(epoch_obj.is_object());
}

#[wasm_bindgen_test]
async fn test_epoch_by_index() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    // Test specific epoch indices
    for index in [0, 1, 10, 100] {
        let epoch = get_epoch_by_index(&sdk, index).await
            .expect(&format!("Failed to get epoch {}", index));
        
        assert_eq!(epoch.index(), index);
        
        // Verify calculations
        let blocks_per_epoch = calculate_epoch_blocks("testnet").unwrap() as u64;
        assert_eq!(epoch.start_block_height(), index as u64 * blocks_per_epoch);
    }
}

#[wasm_bindgen_test]
async fn test_epoch_blocks_calculation() {
    start().await.expect("Failed to start WASM");
    
    // Test different networks
    assert_eq!(calculate_epoch_blocks("mainnet").unwrap(), 1152);
    assert_eq!(calculate_epoch_blocks("testnet").unwrap(), 900);
    assert_eq!(calculate_epoch_blocks("devnet").unwrap(), 20);
    
    // Test invalid network
    assert!(calculate_epoch_blocks("invalid").is_err());
}

#[wasm_bindgen_test]
async fn test_evonodes_retrieval() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    // Get current evonodes
    let evonodes_js = get_current_evonodes(&sdk).await
        .expect("Failed to get current evonodes");
    
    let evonodes = evonodes_js.dyn_ref::<Array>()
        .expect("Evonodes should be an array");
    
    assert!(evonodes.length() > 0);
    
    // Check first evonode structure
    if evonodes.length() > 0 {
        let first_node = evonodes.get(0);
        let node_obj = first_node.dyn_ref::<Object>()
            .expect("Evonode should be an object");
        
        // Verify required fields exist
        assert!(Reflect::has(node_obj, &"proTxHash".into()).unwrap());
        assert!(Reflect::has(node_obj, &"ownerAddress".into()).unwrap());
        assert!(Reflect::has(node_obj, &"votingAddress".into()).unwrap());
        assert!(Reflect::has(node_obj, &"isHPMN".into()).unwrap());
        assert!(Reflect::has(node_obj, &"platformP2PPort".into()).unwrap());
        assert!(Reflect::has(node_obj, &"platformHTTPPort".into()).unwrap());
        assert!(Reflect::has(node_obj, &"nodeIP".into()).unwrap());
    }
}

#[wasm_bindgen_test]
async fn test_evonodes_for_specific_epoch() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("mainnet".to_string(), None).expect("Failed to create SDK");
    
    // Test different epochs have different node counts
    let epoch1_nodes = get_evonodes_for_epoch(&sdk, 1).await.unwrap();
    let epoch2_nodes = get_evonodes_for_epoch(&sdk, 2).await.unwrap();
    
    let array1 = epoch1_nodes.dyn_ref::<Array>().unwrap();
    let array2 = epoch2_nodes.dyn_ref::<Array>().unwrap();
    
    // Mainnet should have base 100 nodes + variation
    assert!(array1.length() >= 100);
    assert!(array2.length() >= 100);
}

#[wasm_bindgen_test]
async fn test_evonode_by_pro_tx_hash() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    // Test valid ProTxHash (32 bytes)
    let pro_tx_hash = vec![1u8; 32];
    let evonode = get_evonode_by_pro_tx_hash(&sdk, pro_tx_hash.clone()).await
        .expect("Failed to get evonode by ProTxHash");
    
    assert_eq!(evonode.pro_tx_hash(), pro_tx_hash);
    assert!(evonode.owner_address().starts_with("yT")); // Testnet address
    assert!(evonode.voting_address().starts_with("yT"));
    assert_eq!(evonode.platform_http_port(), 443);
    
    // Test invalid ProTxHash length
    let invalid_hash = vec![1u8; 16];
    assert!(get_evonode_by_pro_tx_hash(&sdk, invalid_hash).await.is_err());
}

#[wasm_bindgen_test]
async fn test_current_quorum() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("devnet".to_string(), None).expect("Failed to create SDK");
    
    let quorum_js = get_current_quorum(&sdk).await
        .expect("Failed to get current quorum");
    
    let quorum = quorum_js.dyn_ref::<Object>()
        .expect("Quorum should be an object");
    
    // Verify quorum structure
    assert!(Reflect::has(quorum, &"epochIndex".into()).unwrap());
    assert!(Reflect::has(quorum, &"threshold".into()).unwrap());
    assert!(Reflect::has(quorum, &"totalMembers".into()).unwrap());
    assert!(Reflect::has(quorum, &"members".into()).unwrap());
    
    // Check threshold calculation
    let total_members = Reflect::get(quorum, &"totalMembers".into()).unwrap()
        .as_f64().unwrap() as u32;
    let threshold = Reflect::get(quorum, &"threshold".into()).unwrap()
        .as_f64().unwrap() as u32;
    
    // Threshold should be 2/3 + 1 of quorum
    assert_eq!(threshold, (total_members * 2 / 3) + 1);
}

#[wasm_bindgen_test]
async fn test_estimate_next_epoch_time() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    let current_block = 1000;
    let estimate_js = estimate_next_epoch_time(&sdk, current_block).await
        .expect("Failed to estimate next epoch time");
    
    let estimate = estimate_js.dyn_ref::<Object>()
        .expect("Estimate should be an object");
    
    // Verify estimate structure
    assert!(Reflect::has(estimate, &"blocksRemaining".into()).unwrap());
    assert!(Reflect::has(estimate, &"minutesRemaining".into()).unwrap());
    assert!(Reflect::has(estimate, &"estimatedTimeMs".into()).unwrap());
    
    let blocks_remaining = Reflect::get(estimate, &"blocksRemaining".into()).unwrap()
        .as_f64().unwrap() as u32;
    let minutes_remaining = Reflect::get(estimate, &"minutesRemaining".into()).unwrap()
        .as_f64().unwrap();
    
    // Verify calculations
    assert!(blocks_remaining > 0);
    assert_eq!(minutes_remaining, blocks_remaining as f64 * 2.5);
}

#[wasm_bindgen_test]
async fn test_epoch_for_block_height() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    let blocks_per_epoch = calculate_epoch_blocks("testnet").unwrap() as u64;
    
    // Test various block heights
    for (block_height, expected_epoch) in [
        (0, 0),
        (blocks_per_epoch - 1, 0),
        (blocks_per_epoch, 1),
        (blocks_per_epoch * 10 + 50, 10),
    ] {
        let epoch = get_epoch_for_block_height(&sdk, block_height).await
            .expect(&format!("Failed to get epoch for block {}", block_height));
        
        assert_eq!(epoch.index(), expected_epoch);
    }
}

#[wasm_bindgen_test]
async fn test_validator_set_changes() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    // Test valid range
    let changes_js = get_validator_set_changes(&sdk, 1, 3).await
        .expect("Failed to get validator set changes");
    
    let changes = changes_js.dyn_ref::<Object>()
        .expect("Changes should be an object");
    
    // Verify structure
    assert!(Reflect::has(changes, &"fromEpoch".into()).unwrap());
    assert!(Reflect::has(changes, &"toEpoch".into()).unwrap());
    assert!(Reflect::has(changes, &"added".into()).unwrap());
    assert!(Reflect::has(changes, &"removed".into()).unwrap());
    assert!(Reflect::has(changes, &"addedCount".into()).unwrap());
    assert!(Reflect::has(changes, &"removedCount".into()).unwrap());
    
    let from_epoch = Reflect::get(changes, &"fromEpoch".into()).unwrap()
        .as_f64().unwrap() as u32;
    let to_epoch = Reflect::get(changes, &"toEpoch".into()).unwrap()
        .as_f64().unwrap() as u32;
    
    assert_eq!(from_epoch, 1);
    assert_eq!(to_epoch, 3);
    
    // Test invalid range
    assert!(get_validator_set_changes(&sdk, 5, 3).await.is_err());
}

#[wasm_bindgen_test]
async fn test_epoch_stats() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("mainnet".to_string(), None).expect("Failed to create SDK");
    
    let stats_js = get_epoch_stats(&sdk, 5).await
        .expect("Failed to get epoch stats");
    
    let stats = stats_js.dyn_ref::<Object>()
        .expect("Stats should be an object");
    
    // Verify all stats fields
    assert!(Reflect::has(stats, &"epochIndex".into()).unwrap());
    assert!(Reflect::has(stats, &"startBlockHeight".into()).unwrap());
    assert!(Reflect::has(stats, &"startTime".into()).unwrap());
    assert!(Reflect::has(stats, &"totalEvonodes".into()).unwrap());
    assert!(Reflect::has(stats, &"hpmnCount".into()).unwrap());
    assert!(Reflect::has(stats, &"regularNodeCount".into()).unwrap());
    assert!(Reflect::has(stats, &"feeMultiplier".into()).unwrap());
    
    // Verify HPMN calculation
    let total_nodes = Reflect::get(stats, &"totalEvonodes".into()).unwrap()
        .as_f64().unwrap() as u32;
    let hpmn_count = Reflect::get(stats, &"hpmnCount".into()).unwrap()
        .as_f64().unwrap() as u32;
    let regular_count = Reflect::get(stats, &"regularNodeCount".into()).unwrap()
        .as_f64().unwrap() as u32;
    
    assert_eq!(total_nodes, hpmn_count + regular_count);
}

#[wasm_bindgen_test]
async fn test_epoch_fee_multiplier_patterns() {
    start().await.expect("Failed to start WASM");
    
    let sdk = WasmSdk::new("testnet".to_string(), None).expect("Failed to create SDK");
    
    // Test fee multiplier patterns across epochs
    let mut fee_multipliers = Vec::new();
    
    for epoch_idx in 0..25 {
        let epoch = get_epoch_by_index(&sdk, epoch_idx).await
            .expect(&format!("Failed to get epoch {}", epoch_idx));
        fee_multipliers.push(epoch.fee_multiplier());
    }
    
    // Verify fee multiplier pattern (cycles every 20 epochs)
    assert_eq!(fee_multipliers[0], fee_multipliers[20]); // Same phase
    assert!(fee_multipliers[10] > fee_multipliers[0]); // Higher congestion
    assert!(fee_multipliers[15] > fee_multipliers[0]); // Peak congestion
}

#[wasm_bindgen_test]
async fn test_network_specific_evonodes() {
    start().await.expect("Failed to start WASM");
    
    // Test different networks have different node counts
    for (network, min_nodes) in [("mainnet", 100), ("testnet", 50), ("devnet", 10)] {
        let sdk = WasmSdk::new(network.to_string(), None)
            .expect(&format!("Failed to create {} SDK", network));
        
        let evonodes_js = get_evonodes_for_epoch(&sdk, 0).await
            .expect(&format!("Failed to get {} evonodes", network));
        
        let evonodes = evonodes_js.dyn_ref::<Array>().unwrap();
        assert!(evonodes.length() >= min_nodes);
    }
}