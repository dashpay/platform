//! Token tests for rs-sdk-ffi

use crate::config::Config;
use crate::ffi_utils::*;
use rs_sdk_ffi::*;
use std::ffi::CString;
use std::ptr;

/// Test fetching token info
#[test]
#[ignore = "This test needs to be updated to use identity-based token queries"]
fn test_token_info() {
    setup_logs();
    
    let _handle = create_test_sdk_handle("test_token_info");
    
    // NOTE: The token info function requires an identity ID and token IDs
    // This test needs to be rewritten to fetch identity token info
}

/// Test fetching token contract info
#[test]
fn test_token_contract_info() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_token_contract_info");
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    unsafe {
        let result = dash_sdk_token_get_contract_info(handle, token_contract_id.as_ptr());
        
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                
                // Should have contract info
                assert!(json.get("contract").is_some(), "Should have contract field");
                
                // If it has token info
                if json.get("token_info").is_some() {
                    let token_info = json.get("token_info").unwrap();
                    assert!(token_info.get("name").is_some(), "Token info should have name");
                    assert!(token_info.get("symbol").is_some(), "Token info should have symbol");
                }
            }
            Ok(None) => {
                // Contract not found is also valid
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching token balance for an identity
#[test]
fn test_token_balance() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_token_balance");
    
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    let identity_id = to_c_string(&cfg.existing_identity_id);
    
    unsafe {
        let result = dash_sdk_identity_fetch_token_balances(
            handle,
            identity_id.as_ptr(),
            token_contract_id.as_ptr()
        );
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        // The response should be a map of token IDs to balances
        assert!(json.get("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").is_some(), 
               "Should have entry for the token");
        
        let balance = json.get("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec").unwrap();
        assert!(balance.is_string() || balance.is_number(), 
               "Balance should be a string or number, got: {:?}", balance);
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching token balances for multiple identities
#[test]
fn test_token_identities_balances() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_token_identities_balances");
    
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    // Create array of identity IDs
    let identity_ids_json = format!(
        r#"["{}","1111111111111111111111111111111111111111111"]"#,
        cfg.existing_identity_id
    );
    let identity_ids = to_c_string(&identity_ids_json);
    
    unsafe {
        let result = dash_sdk_identities_fetch_token_balances(
            handle,
            identity_ids.as_ptr(),
            token_contract_id.as_ptr()
        );
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        
        // Should have entries for each identity ID
        assert!(json.get(&cfg.existing_identity_id).is_some(), 
               "Should have entry for existing identity");
        assert!(json.get("1111111111111111111111111111111111111111111").is_some(),
               "Should have entry for non-existing identity");
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching all token balances for an identity
#[test]
fn test_identity_token_balances() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_identity_token_balances");
    
    let identity_id = to_c_string(&cfg.existing_identity_id);
    // For testing, we'll use a dummy token ID list
    let token_ids = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    unsafe {
        let result = dash_sdk_token_get_identity_balances(
            handle,
            identity_id.as_ptr(),
            token_ids.as_ptr()
        );
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        assert!(json.get("balances").is_some(), "Should have balances field");
        
        let balances = json.get("balances").unwrap();
        assert!(balances.is_array(), "Balances should be an array");
        
        // Each balance entry should have token info and balance
        if let Some(balances_array) = balances.as_array() {
            for balance_entry in balances_array {
                assert!(balance_entry.get("token_contract_id").is_some(), 
                       "Balance entry should have token_contract_id");
                assert!(balance_entry.get("balance").is_some(), 
                       "Balance entry should have balance");
            }
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching total supply for a token
#[test]
fn test_token_total_supply() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_token_total_supply");
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    unsafe {
        let result = dash_sdk_token_get_total_supply(handle, token_contract_id.as_ptr());
        
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("total_supply").is_some(), "Should have total_supply field");
                
                let total_supply = json.get("total_supply").unwrap();
                assert!(total_supply.is_string() || total_supply.is_number(),
                       "Total supply should be a string or number");
            }
            Ok(None) => {
                // Token might not exist
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching token status
#[test]
fn test_token_status() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_token_status");
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    unsafe {
        let result = dash_sdk_token_get_statuses(handle, token_contract_id.as_ptr());
        
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                
                // Should have status fields
                assert!(json.get("status").is_some(), "Should have status field");
                assert!(json.get("is_locked").is_some(), "Should have is_locked field");
                assert!(json.get("circulating_supply").is_some(), "Should have circulating_supply field");
            }
            Ok(None) => {
                // Token might not exist
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching direct purchase prices
#[test]
fn test_token_direct_purchase_prices() {
    setup_logs();
    
    let handle = create_test_sdk_handle("test_token_direct_purchase_prices");
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    unsafe {
        let result = dash_sdk_token_get_direct_purchase_prices(handle, token_contract_id.as_ptr());
        
        match parse_string_result(result) {
            Ok(Some(json_str)) => {
                let json = parse_json_result(&json_str).expect("valid JSON");
                assert!(json.is_object(), "Expected object, got: {:?}", json);
                assert!(json.get("prices").is_some(), "Should have prices field");
                
                let prices = json.get("prices").unwrap();
                assert!(prices.is_array(), "Prices should be an array");
            }
            Ok(None) => {
                // Token might not have direct purchase enabled
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    destroy_test_sdk_handle(handle);
}

/// Test fetching token info for multiple identities
#[test]
fn test_token_identities_token_infos() {
    setup_logs();
    
    let cfg = Config::new();
    let handle = create_test_sdk_handle("test_token_identities_token_infos");
    
    let token_contract_id = to_c_string("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec");
    
    // Create array of identity IDs
    let identity_ids_json = format!(
        r#"["{}","1111111111111111111111111111111111111111111"]"#,
        cfg.existing_identity_id
    );
    let identity_ids = to_c_string(&identity_ids_json);
    
    unsafe {
        let result = dash_sdk_identities_fetch_token_infos(
            handle,
            identity_ids.as_ptr(),
            token_contract_id.as_ptr()
        );
        
        let json_str = assert_success_with_data(result);
        let json = parse_json_result(&json_str).expect("valid JSON");
        
        assert!(json.is_object(), "Expected object, got: {:?}", json);
        
        // Should have entries for each identity
        assert!(json.get(&cfg.existing_identity_id).is_some(),
               "Should have entry for existing identity");
    }

    destroy_test_sdk_handle(handle);
}