//! Identity tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;

/// Test fetching a non-existent identity
#[test]
fn test_identity_read_not_found() {
    setup_logs();

    let handle = create_test_sdk_handle("test_identity_read_not_found");
    let non_existent_id = to_c_string("1111111111111111111111111111111111111111111");

    unsafe {
        let result = dash_sdk_identity_fetch(handle, non_existent_id.as_ptr());
        assert_success_none(result);
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching an existing identity
#[test]
fn test_identity_read() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_identity_read");
    let id_cstring = to_c_string(&cfg.existing_identity_id);

    unsafe {
        let result = dash_sdk_identity_fetch(handle, id_cstring.as_ptr());
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // Verify we got an identity back
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("id").is_some(), "Identity should have an id field");
        assert!(
            json.get("publicKeys").is_some(),
            "Identity should have publicKeys field"
        );
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching many identities
#[test]
#[ignore = "fetch_many function not available in current SDK"]
fn test_identity_fetch_many() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_identity_read_many");

    let existing_id = cfg.existing_identity_id;
    let non_existent_id = "1111111111111111111111111111111111111111111";

    // Create JSON array of IDs
    let ids_json = format!(r#"["{}","{}"]"#, existing_id, non_existent_id);
    let ids_cstring = to_c_string(&ids_json);

    unsafe {
        // Note: fetch_many function is not available in current SDK
        // We would need to fetch identities one by one
        return;
    }
}

/// Test fetching identity balance
#[test]
fn test_identity_balance() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_identity_balance");
    let id_cstring = to_c_string(&cfg.existing_identity_id);

    unsafe {
        let result = dash_sdk_identity_fetch_balance(handle, id_cstring.as_ptr());
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // Verify we got a balance response
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("balance").is_some(), "Should have balance field");

        let balance = json.get("balance").unwrap();
        assert!(balance.is_number(), "Balance should be a number");
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching identity balance revision
#[test]
fn test_identity_balance_revision() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_identity_balance_and_revision");
    let id_cstring = to_c_string(&cfg.existing_identity_id);

    unsafe {
        let result = dash_sdk_identity_fetch_balance_and_revision(handle, id_cstring.as_ptr());
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // Verify we got balance and revision
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("balance").is_some(), "Should have balance field");
        assert!(json.get("revision").is_some(), "Should have revision field");

        let balance = json.get("balance").unwrap();
        assert!(balance.is_number(), "Balance should be a number");

        let revision = json.get("revision").unwrap();
        assert!(revision.is_number(), "Revision should be a number");
    }

    destroy_test_sdk_handle(handle);
}

/// Test resolving identity by alias
#[test]
fn test_identity_resolve_by_alias() {
    setup_logs();

    let handle = create_test_sdk_handle("test_identity_read_by_dpns_name");
    let alias_cstring = to_c_string("dash");

    unsafe {
        let result = dash_sdk_identity_resolve_name(handle, alias_cstring.as_ptr());

        // This might return None if the alias doesn't exist in test vectors
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("identity").is_some(), "Should have identity field");
                assert!(json.get("alias").is_some(), "Should have alias field");
            }
            Ok(None) => {
                // Alias not found is also valid for test vectors
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching identity keys
#[test]
fn test_identity_fetch_keys() {
    setup_logs();

    let cfg = Config::new();
    let handle = create_test_sdk_handle("identity_keys");
    let id_cstring = to_c_string(&cfg.existing_identity_id);

    // Fetch all keys
    let key_ids_json = "[]"; // empty array means fetch all
    let key_ids_cstring = to_c_string(key_ids_json);

    unsafe {
        let result = dash_sdk_identity_fetch_public_keys(handle, id_cstring.as_ptr());

        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");

        // Verify we got keys back
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("keys").is_some(), "Should have keys field");

        let keys = json.get("keys").unwrap();
        assert!(keys.is_array(), "Keys should be an array");

        // If we have keys, verify they have the expected structure
        if let Some(keys_array) = keys.as_array() {
            if !keys_array.is_empty() {
                let first_key = &keys_array[0];
                assert!(first_key.get("id").is_some(), "Key should have id field");
                assert!(
                    first_key.get("type").is_some(),
                    "Key should have type field"
                );
                assert!(
                    first_key.get("purpose").is_some(),
                    "Key should have purpose field"
                );
                assert!(
                    first_key.get("securityLevel").is_some(),
                    "Key should have securityLevel field"
                );
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching identity by public key hash
#[test]
fn test_identity_fetch_by_public_key_hash() {
    setup_logs();

    let handle = create_test_sdk_handle("test_identity_read_by_public_key_hash");

    // This is a test public key hash - may or may not exist in test vectors
    let test_key_hash = "0000000000000000000000000000000000000000";
    let key_hash_cstring = to_c_string(test_key_hash);

    unsafe {
        let result = dash_sdk_identity_fetch_by_public_key_hash(handle, key_hash_cstring.as_ptr());

        // This test may return None if no identity has this key hash
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("identity").is_some(), "Should have identity field");
            }
            Ok(None) => {
                // Not found is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}
