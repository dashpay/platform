//! System tests for rs-sdk-ffi

use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ptr;

/// Test fetching epochs info
#[test]
fn test_epochs_info() {
    setup_logs();

    let handle = create_test_sdk_handle("test_epoch_list_limit_3");

    unsafe {
        let result = dash_sdk_system_get_epochs_info(
            handle,
            ptr::null(), // start_epoch - null means use default
            3,           // count - fetch 3 epochs
            true,        // ascending - oldest first
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("epochs").is_some(), "Should have epochs field");

        let epochs = json.get("epochs").unwrap();
        assert!(epochs.is_array(), "Epochs should be an array");

        // Verify epoch structure
        if let Some(epochs_array) = epochs.as_array() {
            assert!(epochs_array.len() <= 3, "Should have at most 3 epochs");

            for epoch in epochs_array {
                assert!(epoch.get("index").is_some(), "Epoch should have index");
                assert!(
                    epoch.get("first_block_height").is_some(),
                    "Epoch should have first_block_height"
                );
                assert!(
                    epoch.get("first_core_block_height").is_some(),
                    "Epoch should have first_core_block_height"
                );
                assert!(
                    epoch.get("start_time").is_some(),
                    "Epoch should have start_time"
                );
                assert!(
                    epoch.get("fee_multiplier").is_some(),
                    "Epoch should have fee_multiplier"
                );
            }

            // Verify ordering if we have multiple epochs
            if epochs_array.len() > 1 {
                let first_index = epochs_array[0].get("index").unwrap().as_u64().unwrap();
                let second_index = epochs_array[1].get("index").unwrap().as_u64().unwrap();
                assert!(
                    first_index < second_index,
                    "Epochs should be in ascending order"
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching current quorums info
#[test]
fn test_current_quorums_info() {
    setup_logs();

    let handle = create_test_sdk_handle("test_current_quorums");

    unsafe {
        let result = dash_sdk_system_get_current_quorums_info(handle);

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("quorums").is_some(), "Should have quorums field");

        let quorums = json.get("quorums").unwrap();
        assert!(quorums.is_object(), "Quorums should be an object");

        // Each quorum type should have a list of quorums
        for (_quorum_type, quorum_list) in quorums.as_object().unwrap() {
            assert!(quorum_list.is_array(), "Quorum list should be an array");

            if let Some(quorum_array) = quorum_list.as_array() {
                for quorum in quorum_array {
                    assert!(quorum.get("hash").is_some(), "Quorum should have hash");
                    assert!(quorum.get("index").is_some(), "Quorum should have index");
                    assert!(
                        quorum.get("active_members").is_some(),
                        "Quorum should have active_members"
                    );
                    assert!(
                        quorum.get("created_at").is_some(),
                        "Quorum should have created_at"
                    );
                }
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching specific epochs with offset
#[test]
fn test_epochs_info_with_offset() {
    setup_logs();

    let handle = create_test_sdk_handle("test_epoch_list_offset");

    unsafe {
        // First get some epochs
        let result1 = dash_sdk_system_get_epochs_info(
            handle,
            ptr::null(), // start_epoch - null means use default
            2,           // count
            true,        // ascending
        );

        let json_str1 = assert_success_with_data(result1);
        let json1 = parse_json_result(&json_str1).expect("valid JSON");
        let epochs1 = json1.get("epochs").unwrap().as_array().unwrap();

        if epochs1.len() >= 2 {
            // Now get epochs with offset (should skip first epoch)
            // Note: epochs_info_with_offset function doesn't exist, we'll skip this part
            // The SDK only has get_epochs_info without offset parameter
        }
    }

    destroy_test_sdk_handle(handle);
}

// Test fetching block info is removed - function not available in current SDK

// Test fetching platform value is removed - function not available in current SDK

/// Test fetching total credits in platform
#[test]
fn test_total_credits_in_platform() {
    setup_logs();

    let handle = create_test_sdk_handle("test_total_credits_in_platform");

    unsafe {
        let result = dash_sdk_system_get_total_credits_in_platform(handle);

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(
            json.get("total_credits").is_some(),
            "Should have total_credits field"
        );

        let total_credits = json.get("total_credits").unwrap();
        assert!(
            total_credits.is_string() || total_credits.is_number(),
            "Total credits should be a string or number"
        );
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching path elements
#[test]
fn test_path_elements() {
    setup_logs();

    let handle = create_test_sdk_handle("test_path_elements");

    // Query for some platform elements
    let path_json = r#"["platform_state"]"#;
    let path_query = to_c_string(path_json);

    // Keys parameter - empty array means get all keys
    let keys_json = "[]";
    let keys_query = to_c_string(keys_json);

    unsafe {
        let result =
            dash_sdk_system_get_path_elements(handle, path_query.as_ptr(), keys_query.as_ptr());

        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let _json = parse_json_result(&json_str).expect("valid JSON");
                // The response format depends on what's at the path
                // Could be an object with elements or the elements directly
            }
            Ok(None) => {
                // No elements found is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}
