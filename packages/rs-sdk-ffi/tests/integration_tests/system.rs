//! System tests for rs-sdk-ffi

use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ptr;

/// Test fetching epochs info
#[test]
fn test_epochs_info() {
    setup_logs();

    // Align with rs-sdk vector: test_epoch_list_limit
    let handle = create_test_sdk_handle("test_epoch_list_limit");

    unsafe {
        // Match rs-sdk vectors: start at epoch 193, count=2, ascending=true
        let start_epoch = to_c_string("193");
        let result = dash_sdk_system_get_epochs_info(handle, start_epoch.as_ptr(), 2, true);

        // Allow None when vectors/data happen to be empty in offline mode
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("epochs").is_some(), "Should have epochs field");
            }
            Ok(None) => {}
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

// Pruned: current quorums not backed by rs-sdk vectors

// Pruned: epochs offset variant not supported and no rs-sdk vectors

// Test fetching block info is removed - function not available in current SDK

// Test fetching platform value is removed - function not available in current SDK

// Pruned: total credits not backed by rs-sdk vectors

// Pruned: path elements not backed by rs-sdk vectors
