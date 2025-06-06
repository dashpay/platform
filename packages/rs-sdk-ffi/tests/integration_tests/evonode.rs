//! Evonode tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ffi::CString;
use std::ptr;

/// Test fetching proposed epoch blocks by range
#[test]
fn test_evonode_proposed_epoch_blocks_by_range() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_proposed_blocks");
    
    unsafe {
        let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
            handle,
            0,           // epoch (0 = current)
            10,          // limit
            ptr::null(), // start_after
            ptr::null()  // start_at
        );
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        // The response is an array of evonode proposed block counts
        assert!(json.is_array(), "Expected array, got: {:?}", json);
        
        // Verify proposed blocks structure
        if let Some(blocks_array) = json.as_array() {
            for block in blocks_array {
                assert!(block.is_object(), "Each block should be an object");
                assert!(block.get("pro_tx_hash").is_some(), "Block should have pro_tx_hash");
                assert!(block.get("count").is_some(), "Block should have count");
                
                let pro_tx_hash = block.get("pro_tx_hash").unwrap();
                assert!(pro_tx_hash.is_string(), "pro_tx_hash should be a string");
                
                let count = block.get("count").unwrap();
                assert!(count.is_number(), "Count should be a number");
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching proposed blocks by specific IDs
#[test]
fn test_evonode_proposed_epoch_blocks_by_ids() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_proposed_blocks_by_ids");
    
    // Create a JSON array with the masternode ProTxHash
    let ids_json = format!("[\"{}\"]", cfg.masternode_owner_pro_reg_tx_hash);
    let ids_cstring = to_c_string(&ids_json);
    
    unsafe {
        let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(
            handle,
            0,                  // epoch (0 = current)
            ids_cstring.as_ptr() // IDs as JSON array
        );
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        // The response is an array of evonode proposed block counts
        assert!(json.is_array(), "Expected array, got: {:?}", json);
        
        // Verify proposed blocks structure
        if let Some(blocks_array) = json.as_array() {
            for block in blocks_array {
                assert!(block.is_object(), "Each block should be an object");
                assert!(block.get("pro_tx_hash").is_some(), "Block should have pro_tx_hash");
                assert!(block.get("count").is_some(), "Block should have count");
                
                let pro_tx_hash = block.get("pro_tx_hash").unwrap();
                assert!(pro_tx_hash.is_string(), "pro_tx_hash should be a string");
                
                // If we have blocks, verify they match our requested IDs
                if let Some(hash_str) = pro_tx_hash.as_str() {
                    assert_eq!(hash_str, cfg.masternode_owner_pro_reg_tx_hash,
                              "Block pro_tx_hash should match requested ID");
                }
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

// Test fetching evonode status is removed - function not available in current SDK

// Test fetching multiple evonodes status is removed - function not available in current SDK

// Test fetching proposed blocks in range is removed - use test_evonode_proposed_epoch_blocks_by_range instead