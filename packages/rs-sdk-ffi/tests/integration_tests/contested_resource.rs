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
    let identity_id = to_c_string(&cfg.existing_identity_id);

    unsafe {
        let result = dash_sdk_contested_resource_get_identity_votes(
            handle,
            identity_id.as_ptr(),
            10,   // limit
            0,    // offset
            true, // order_ascending
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // Verify we got a votes response
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("votes").is_some(), "Should have votes field");

        let votes = json.get("votes").unwrap();
        assert!(votes.is_array(), "Votes should be an array");
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

    // Search for contested resources
    let index_values_json = r#"["dash", "test"]"#;
    let index_values = to_c_string(index_values_json);

    unsafe {
        let result = dash_sdk_contested_resource_get_resources(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            ptr::null(), // start_index_values
            10,          // limit
            true,        // order_ascending
        );

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // Verify response structure
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(
            json.get("contested_resources").is_some(),
            "Should have contested_resources field"
        );

        let resources = json.get("contested_resources").unwrap();
        assert!(
            resources.is_array(),
            "Contested resources should be an array"
        );
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

    // Look for dash.test or similar contested resource
    let index_values_json = r#"["dash", "test"]"#;
    let index_values = to_c_string(index_values_json);

    // DocumentsAndVoteTally result type
    unsafe {
        let result = dash_sdk_contested_resource_get_vote_state(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            2,     // result_type: 2=DOCUMENTS_AND_VOTE_TALLY
            false, // allow_include_locked_and_abstaining_vote_tally
            10,    // count
        );

        // This might return None if no contested resource exists
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);

                // Should have vote tally info
                assert!(
                    json.get("abstain_vote_tally").is_some(),
                    "Should have abstain_vote_tally"
                );
                assert!(
                    json.get("lock_vote_tally").is_some(),
                    "Should have lock_vote_tally"
                );
                assert!(json.get("contenders").is_some(), "Should have contenders");
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

    let index_values_json = r#"["dash", "test"]"#;
    let index_values = to_c_string(index_values_json);

    let contender_id = to_c_string(&cfg.existing_identity_id);

    unsafe {
        let result = dash_sdk_contested_resource_get_voters_for_identity(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            contender_id.as_ptr(),
            10,   // count
            true, // order_ascending
        );

        // This might return None if the identity is not a contender
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("voters").is_some(), "Should have voters field");

                let voters = json.get("voters").unwrap();
                assert!(voters.is_array(), "Voters should be an array");
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

    let index_values_json = r#"["dash"]"#;
    let index_values = to_c_string(index_values_json);

    // OnlyVoteTally result type - simpler response
    unsafe {
        let result = dash_sdk_contested_resource_get_vote_state(
            handle,
            contract_id.as_ptr(),
            document_type.as_ptr(),
            index_name.as_ptr(),
            index_values.as_ptr(),
            1,    // result_type: 1=VOTE_TALLY
            true, // allow_include_locked_and_abstaining_vote_tally
            5,    // count
        );

        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);

                // With OnlyVoteTally, should have vote tallies but no documents
                assert!(
                    json.get("abstain_vote_tally").is_some(),
                    "Should have abstain_vote_tally"
                );
                assert!(
                    json.get("lock_vote_tally").is_some(),
                    "Should have lock_vote_tally"
                );

                // Should not have contenders with documents
                if let Some(contenders) = json.get("contenders") {
                    if let Some(contenders_array) = contenders.as_array() {
                        for contender in contenders_array {
                            assert!(
                                contender.get("document").is_none()
                                    || contender.get("document").unwrap().is_null(),
                                "OnlyVoteTally should not include documents"
                            );
                        }
                    }
                }
            }
            Ok(None) => {
                // No contested resource is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}
