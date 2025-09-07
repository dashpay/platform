//! FFI-specific test utilities

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use rs_sdk_ffi::*;
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;

/// Create an SDK handle for testing using the mock mode with offline test vectors
pub fn create_test_sdk_handle(namespace: &str) -> *const SDKHandle {
    // Use the rs-sdk test vectors directory
    let base_dump_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("rs-sdk")
        .join("tests")
        .join("vectors");

    // Some historical test namespaces differ from directory names in vectors.
    // Map known mismatches and fall back gracefully if a directory is missing.
    fn map_namespace(ns: &str) -> &str {
        match ns {
            // Contested resource mappings
            "test_contested_resources" => "test_contested_resources_ok",
            "test_contested_resource_vote_state" => "contested_resource_vote_states_ok",
            "test_contested_resource_voters_for_identity" => {
                "contested_resource_voters_for_existing_contestant"
            }
            "test_contested_resources_fields_limit" => "contested_resource_vote_states_with_limit",

            // Document queries
            // Route both to a directory that contains GetDataContract + DocumentQuery vectors
            "document_list_dpns_where_domain_startswith" => "document_list_document_query",
            "test_document_read_complex" => "document_list_document_query",
            "test_document_list_empty_where" => "document_list_document_query",
            "document_read_no_document" => "document_read_no_document",

            // Epoch/voting
            "test_epoch_list_limit_3" => "test_epoch_list_limit",
            "test_epoch_list_limit" => "test_epoch_list_limit",
            "test_vote_polls_by_end_date" => "vote_polls_by_ts_ok",
            "test_vote_polls_by_end_date_range" => "vote_polls_by_ts_limit",
            "test_vote_polls_paginated" => "vote_polls_by_ts_limit",
            "test_vote_polls_descending" => "vote_polls_by_ts_order",
            "test_active_vote_polls" => "vote_polls_by_ts_ok",

            // Data contract history
            "test_data_contract_history" => "test_data_contract_history_read",

            // Identity mappings
            "test_identity_balance" => "test_identity_balance_read",
            "test_identity_balance_revision" => "test_identity_balance_revision_read",
            "test_identity_balance_and_revision" => "test_identity_balance_revision_read",
            "test_identity_fetch_by_public_key_hash" => "test_identity_read_by_key",
            "test_identity_read_by_public_key_hash" => "test_identity_read_by_key",
            "test_identity_fetch_keys" => "test_identity_public_keys_all_read",
            "identity_keys" => "test_identity_public_keys_all_read",
            "test_identity_read_by_dpns_name" => "document_list_document_query",
            // Not-found variants may not have a dedicated dir; fallback will handle it

            // Token mappings
            "test_token_identities_token_infos" => "test_identities_token_infos",
            "test_token_direct_purchase_prices" => "test_direct_prices_tokens_ok",
            "test_token_identities_balances" => "test_multiple_identities_token_balances",
            "test_identity_token_balances" => "test_multiple_identity_token_balances",

            // Protocol version mappings
            "test_version_upgrade_state" => "test_protocol_version_vote_count",
            "test_version_upgrade_vote_status" => "test_protocol_version_votes_limit_2",

            // System mappings
            "test_current_quorums" => "test_current_quorums",
            "test_total_credits_in_platform" => "test_total_credits_in_platform",
            "test_path_elements" => "test_path_elements",

            _ => ns,
        }
    }

    let dump_dir = if namespace.is_empty() {
        base_dump_dir.clone()
    } else {
        let mapped = map_namespace(namespace);
        base_dump_dir.join(mapped.replace(' ', "_"))
    };

    // If the mapped directory does not exist, fall back to base vectors dir
    let dump_dir = if fs::metadata(&dump_dir).is_ok() {
        dump_dir
    } else {
        eprintln!(
            "⚠️ Integration test vectors directory not found: {} — falling back to base vectors at {}",
            dump_dir.display(),
            base_dump_dir.display()
        );
        base_dump_dir
    };

    let dump_dir_str = CString::new(dump_dir.to_string_lossy().as_ref()).unwrap();

    unsafe {
        let handle = dash_sdk_create_handle_with_mock(dump_dir_str.as_ptr());
        if handle.is_null() {
            panic!("Failed to create mock SDK handle");
        }
        handle as *const SDKHandle
    }
}

/// Destroy an SDK handle
pub fn destroy_test_sdk_handle(handle: *const SDKHandle) {
    unsafe {
        dash_sdk_destroy(handle as *mut SDKHandle);
    }
}

/// Convert a Rust string to a C string pointer
pub fn to_c_string(s: &str) -> CString {
    CString::new(s).expect("Failed to create CString")
}

/// Convert a C string pointer to a Rust string
pub unsafe fn from_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// Create a valid Base58-encoded 32-byte identifier from a byte pattern
pub fn base58_from_bytes(byte: u8) -> String {
    let id = Identifier::from_bytes(&[byte; 32]).expect("valid identifier bytes");
    id.to_string(Encoding::Base58)
}

/// Convert a hex-encoded 32-byte identifier to Base58 string
pub fn base58_from_hex32(hex_str: &str) -> String {
    let id = Identifier::from_string(hex_str, Encoding::Hex).expect("valid hex identifier");
    id.to_string(Encoding::Base58)
}

/// Parse a DashSDKResult and extract the string data
pub unsafe fn parse_string_result(result: DashSDKResult) -> Result<Option<String>, String> {
    if !result.error.is_null() {
        let error = Box::from_raw(result.error);
        return Err(format!(
            "Error code {}: {}",
            error.code as i32,
            from_c_string(error.message).unwrap_or_default()
        ));
    }

    match result.data_type {
        DashSDKResultDataType::NoData => Ok(None),
        DashSDKResultDataType::String => {
            if result.data.is_null() {
                Ok(None)
            } else {
                let c_str = CStr::from_ptr(result.data as *const c_char);
                let string = c_str.to_string_lossy().into_owned();
                // Free the C string
                dash_sdk_string_free(result.data as *mut c_char);
                Ok(Some(string))
            }
        }
        _ => Err("Unexpected result data type".to_string()),
    }
}

/// Parse a JSON string result
pub fn parse_json_result(json: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))
}

/// Test helper to assert that a result is successful and contains data
pub unsafe fn assert_success_with_data(result: DashSDKResult) -> String {
    let data = parse_string_result(result)
        .expect("Result should be successful")
        .expect("Result should contain data");
    data
}

/// Test helper to assert that a result is successful but contains no data (None)
pub unsafe fn assert_success_none(result: DashSDKResult) {
    let data = parse_string_result(result).expect("Result should be successful");
    assert!(data.is_none(), "Expected None but got data: {:?}", data);
}

/// Test helper to assert that a result is an error
pub unsafe fn assert_error(result: DashSDKResult) {
    assert!(
        parse_string_result(result).is_err(),
        "Expected error but got success"
    );
}

/// Setup logging for tests
pub fn setup_logs() {
    // Initialize logging if needed
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}
