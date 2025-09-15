#![allow(unused_variables)]
//! Contested resource tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ptr;

/// Test fetching identity votes for contested resources
#[test]
fn test_contested_resource_identity_votes() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("contested_resource_identity_votes_ok");
    // Match vectors: identity id equals the masternode proTxHash ([0x06,0x9d,...])
    let identity_id = to_c_string(&base58_from_hex32(&cfg.masternode_owner_pro_reg_tx_hash));

    unsafe {
        let result = dash_sdk_contested_resource_get_identity_votes(
            handle,
            identity_id.as_ptr(),
            0,    // limit = 0 (no limit in vectors)
            0,    // offset = 0 (none)
            true, // order_ascending
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // FFI returns an array of votes
        assert!(json.is_array(), "Expected array, got: {:?}", json);
        if let Some(first) = json.as_array().and_then(|a| a.first()) {
            assert!(first.get("vote_poll_id").is_some());
            assert!(first.get("resource_vote_choice").is_some());
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching contested resources
#[test]
fn test_contested_resources() {
    setup_logs();

    let handle = create_test_sdk_handle("test_contested_resources");

    // DPNS contract for testing contested domains
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    let index_name = to_c_string("parentNameAndLabel");

    // Match vectors: only the parent name value, descending order, no limit
    let start_index_values_json = r#"["dash"]"#;
    let start_index_values = to_c_string(start_index_values_json);

    unsafe {
        let result = dash_sdk_contested_resource_get_resources(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            start_index_values.as_ptr(),
            ptr::null(), // start_index_values
            0,           // count = 0 (null in vectors)
            false,       // order_ascending = false per vectors
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // FFI returns an array of contested resources
        assert!(json.is_array(), "Expected array, got: {:?}", json);
        if let Some(first) = json.as_array().and_then(|a| a.first()) {
            assert!(first.get("id").is_some());
            assert!(first.get("contract_id").is_some());
            assert!(first.get("document_type_name").is_some());
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching vote state for a contested resource
#[test]
fn test_contested_resource_vote_state() {
    setup_logs();

    let handle = create_test_sdk_handle("test_contested_resource_vote_state");

    // DPNS contract
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    let index_name = to_c_string("parentNameAndLabel");

    // Match vectors: look for dash.testname as plain values
    let index_values_json = r#"["dash", "testname"]"#;
    let index_values = to_c_string(index_values_json);

    // DocumentsAndVoteTally result type
    unsafe {
        let result = dash_sdk_contested_resource_get_vote_state(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            2,    // result_type: 2=DOCUMENTS_AND_VOTE_TALLY
            true, // allow_include_locked_and_abstaining_vote_tally per vectors
            0,    // count = 0 (null in vectors)
        );

        // This might return None if no contested resource exists
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                // Should have vote tally info if present
                if let Some(obj) = json.as_object() {
                    if obj.contains_key("abstain_vote_tally") {
                        assert!(obj.get("lock_vote_tally").is_some());
                    }
                }
            }
            Ok(None) => {
                // No contested resource found is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching voters for a specific identity in a contested resource
#[test]
fn test_contested_resource_voters_for_identity() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_contested_resource_voters_for_identity");

    // DPNS contract
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    let index_name = to_c_string("parentNameAndLabel");

    // Match vectors: plain values that the SDK will serialize
    let index_values_json = r#"["dash", "testname"]"#;
    let index_values = to_c_string(index_values_json);

    // Use contestant id from vectors (hex → base58)
    let contender_id = to_c_string(&base58_from_hex32(
        "a496fe4262159124ad8aad5f92a7739650584bbeccfa7dbbd72f8510321c95b2",
    ));

    unsafe {
        let result = dash_sdk_contested_resource_get_voters_for_identity(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            contender_id.as_ptr(),
            0,    // count = 0 (no limit in vectors)
            true, // order_ascending
        );

        // This might return None if the identity is not a contender
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                // FFI returns an array of voters
                assert!(json.is_array(), "Expected array, got: {:?}", json);
                if let Some(first) = json.as_array().and_then(|a| a.first()) {
                    assert!(first.get("voter_id").is_some());
                }
            }
            Ok(None) => {
                // Not a contender is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test complex contested resource vote state query
#[test]
fn test_contested_resource_vote_state_complex() {
    setup_logs();

    let handle = create_test_sdk_handle("test_contested_resources_fields_limit");

    // DPNS contract
    let contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let document_type = to_c_string("domain");
    let index_name = to_c_string("parentNameAndLabel");

    // Match vote_state vector: requires two index values
    let index_values_json = r#"["dash", "testname"]"#;
    let index_values = to_c_string(index_values_json);

    // OnlyVoteTally result type - simpler response
    unsafe {
        let result = dash_sdk_contested_resource_get_vote_state(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            2,    // result_type: 2=DOCUMENTS_AND_VOTE_TALLY
            true, // allow_include_locked_and_abstaining_vote_tally
            2,    // count per vectors
        );

        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);

                // Should have vote tallies present
                assert!(json.get("abstain_vote_tally").is_some());
                assert!(json.get("lock_vote_tally").is_some());
            }
            Ok(None) => {
                // No contested resource is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}
