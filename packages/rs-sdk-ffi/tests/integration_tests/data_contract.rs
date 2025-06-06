//! Data contract tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ffi::CString;

/// Given some dummy data contract ID, when I fetch data contract, I get None because it doesn't exist.
#[test]
fn test_data_contract_read_not_found() {
    setup_logs();

    let handle = create_test_sdk_handle("test_data_contract_read_not_found");
    let non_existent_id = "1111111111111111111111111111111111111111111";
    let id_cstring = to_c_string(non_existent_id);
    
    unsafe {
        let result = dash_sdk_data_contract_fetch(handle, id_cstring.as_ptr());
        assert_success_none(result);
    }

    destroy_test_sdk_handle(handle);
}

/// Given some existing data contract ID, when I fetch data contract, I get the data contract.
#[test]
fn test_data_contract_read() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_data_contract_read");
    let id_cstring = to_c_string(&cfg.existing_data_contract_id);
    
    unsafe {
        let result = dash_sdk_data_contract_fetch(handle, id_cstring.as_ptr());
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        // Verify we got a data contract back
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("id").is_some(), "Data contract should have an id field");
    }

    destroy_test_sdk_handle(handle);
}

/// Given existing and non-existing data contract IDs, when I fetch them, I get the existing data contract.
#[test]
fn test_data_contracts_1_ok_1_nx() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_data_contracts_1_ok_1_nx");
    
    let existing_id = cfg.existing_data_contract_id;
    let non_existent_id = "1111111111111111111111111111111111111111111";
    
    // Create JSON array of IDs
    let ids_json = format!(r#"["{}","{}"]"#, existing_id, non_existent_id);
    let ids_cstring = to_c_string(&ids_json);
    
    unsafe {
        let result = dash_sdk_data_contracts_fetch_many(handle, ids_cstring.as_ptr());
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        // Verify we got an object with our IDs as keys
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        
        // Check existing contract
        let existing_contract = json.get(&existing_id);
        assert!(existing_contract.is_some(), "Should have entry for existing ID");
        assert!(!existing_contract.unwrap().is_null(), "Existing contract should not be null");
        
        // Check non-existing contract
        let non_existing_contract = json.get(non_existent_id);
        assert!(non_existing_contract.is_some(), "Should have entry for non-existing ID");
        assert!(non_existing_contract.unwrap().is_null(), "Non-existing contract should be null");
    }

    destroy_test_sdk_handle(handle);
}

/// Given two non-existing data contract IDs, I get None for both.
#[test]
fn test_data_contracts_2_nx() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_data_contracts_2_nx");
    
    let non_existent_id_1 = "0000000000000000000000000000000000000000000";
    let non_existent_id_2 = "1111111111111111111111111111111111111111111";
    
    // Create JSON array of IDs
    let ids_json = format!(r#"["{}","{}"]"#, non_existent_id_1, non_existent_id_2);
    let ids_cstring = to_c_string(&ids_json);
    
    unsafe {
        let result = dash_sdk_data_contracts_fetch_many(handle, ids_cstring.as_ptr());
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        // Verify we got an object with our IDs as keys
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        
        // Check both are null
        let contract_1 = json.get(non_existent_id_1);
        assert!(contract_1.is_some(), "Should have entry for first ID");
        assert!(contract_1.unwrap().is_null(), "First contract should be null");
        
        let contract_2 = json.get(non_existent_id_2);
        assert!(contract_2.is_some(), "Should have entry for second ID");
        assert!(contract_2.unwrap().is_null(), "Second contract should be null");
    }

    destroy_test_sdk_handle(handle);
}

/// Test data contract history fetch
#[test]
fn test_data_contract_history() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_data_contract_history");
    let id_cstring = to_c_string(&cfg.existing_data_contract_id);
    
    unsafe {
        let result = dash_sdk_data_contract_fetch_history(
            handle,
            id_cstring.as_ptr(),
            10,     // limit
            0,      // offset  
            0       // start_at_ms (0 = no filter)
        );
        
        // This test may return None if the contract has no history
        // or data if history exists
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                // Should have contract_id and history fields
                assert!(json.get("contract_id").is_some(), "Should have contract_id field");
                assert!(json.get("history").is_some(), "Should have history field");
            }
            Ok(None) => {
                // No history is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}