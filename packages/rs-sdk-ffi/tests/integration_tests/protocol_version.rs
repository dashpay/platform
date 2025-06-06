//! Protocol version tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;

/// Test fetching protocol version upgrade state
#[test]
fn test_protocol_version_upgrade_state() {
    setup_logs();

    let handle = create_test_sdk_handle("test_version_upgrade_state");

    unsafe {
        let result = dash_sdk_protocol_version_get_upgrade_state(handle);

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // The response is an array of protocol version upgrade information
        assert!(json.is_array(), "Expected array, got: {:?}", json);

        // Verify upgrade state structure if array is not empty
        if let Some(upgrades_array) = json.as_array() {
            for upgrade in upgrades_array {
                assert!(upgrade.is_object(), "Each upgrade should be an object");
                assert!(
                    upgrade.get("version_number").is_some(),
                    "Should have version_number"
                );
                assert!(
                    upgrade.get("vote_count").is_some(),
                    "Should have vote_count"
                );

                let version_number = upgrade.get("version_number").unwrap();
                assert!(
                    version_number.is_number(),
                    "Version number should be a number"
                );

                let vote_count = upgrade.get("vote_count").unwrap();
                assert!(vote_count.is_number(), "Vote count should be a number");
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching protocol version upgrade vote status
#[test]
fn test_protocol_version_upgrade_vote_status() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_version_upgrade_vote_status");

    // Use the masternode ProTxHash from config
    let pro_tx_hash = to_c_string(&cfg.masternode_owner_pro_reg_tx_hash);

    unsafe {
        let result = dash_sdk_protocol_version_get_upgrade_vote_status(
            handle,
            pro_tx_hash.as_ptr(),
            10, // count
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // The response is an array of masternode protocol version votes
        assert!(json.is_array(), "Expected array, got: {:?}", json);

        // Verify vote status structure if array is not empty
        if let Some(votes_array) = json.as_array() {
            for vote in votes_array {
                assert!(vote.is_object(), "Each vote should be an object");
                assert!(vote.get("pro_tx_hash").is_some(), "Should have pro_tx_hash");
                assert!(vote.get("version").is_some(), "Should have version");

                let pro_tx_hash = vote.get("pro_tx_hash").unwrap();
                assert!(pro_tx_hash.is_string(), "pro_tx_hash should be a string");

                let version = vote.get("version").unwrap();
                assert!(version.is_number(), "Version should be a number");
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

// Test fetching protocol version history is removed - function not available in current SDK

// Test fetching specific protocol version info is removed - function not available in current SDK

// Test fetching all known protocol versions is removed - function not available in current SDK
