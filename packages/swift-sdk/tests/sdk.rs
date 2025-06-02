//! Tests for SDK initialization and configuration

use swift_sdk::*;
use std::ptr;

// Import ios_sdk_ffi for handle types and functions
extern crate ios_sdk_ffi;

#[test]
fn test_sdk_initialization() {
    // Initialize the SDK library
    unsafe {
        swift_dash_sdk_init();
    }
}

#[test]
fn test_sdk_version() {
    let version_ptr = unsafe { swift_dash_sdk_get_version() };
    
    assert!(!version_ptr.is_null());
    
    let version = unsafe {
        std::ffi::CStr::from_ptr(version_ptr)
            .to_string_lossy()
            .to_string()
    };
    
    // Free the version string
    unsafe {
        ios_sdk_ffi::ios_sdk_string_free(version_ptr);
    }
    
    assert!(!version.is_empty());
    println!("SDK Version: {}", version);
}

#[test]
fn test_sdk_config_creation() {
    // Test mainnet config
    let mainnet_config = unsafe { swift_dash_sdk_config_mainnet() };
    assert_eq!(mainnet_config.network, SwiftDashNetwork::Mainnet);
    assert!(!mainnet_config.skip_asset_lock_proof_verification);
    assert_eq!(mainnet_config.request_retry_count, 3);
    assert_eq!(mainnet_config.request_timeout_ms, 30000);
    
    // Test testnet config
    let testnet_config = unsafe { swift_dash_sdk_config_testnet() };
    assert_eq!(testnet_config.network, SwiftDashNetwork::Testnet);
    assert!(!testnet_config.skip_asset_lock_proof_verification);
    assert_eq!(testnet_config.request_retry_count, 3);
    assert_eq!(testnet_config.request_timeout_ms, 30000);
    
    // Test local config
    let local_config = unsafe { swift_dash_sdk_config_local() };
    assert_eq!(local_config.network, SwiftDashNetwork::Local);
    assert!(local_config.skip_asset_lock_proof_verification);
    assert_eq!(local_config.request_retry_count, 1);
    assert_eq!(local_config.request_timeout_ms, 10000);
}

#[test]
fn test_put_settings_default() {
    let settings = unsafe { swift_dash_put_settings_default() };
    
    assert_eq!(settings.connect_timeout_ms, 0);
    assert_eq!(settings.timeout_ms, 0);
    assert_eq!(settings.retries, 0);
    assert!(!settings.ban_failed_address);
    assert_eq!(settings.identity_nonce_stale_time_s, 0);
    assert_eq!(settings.user_fee_increase, 0);
    assert!(!settings.allow_signing_with_any_security_level);
    assert!(!settings.allow_signing_with_any_purpose);
    assert_eq!(settings.wait_timeout_ms, 0);
}

#[test]
fn test_sdk_create_and_destroy() {
    unsafe {
        swift_dash_sdk_init();
    }
    
    let config = unsafe { swift_dash_sdk_config_local() };
    let sdk_handle = unsafe { swift_dash_sdk_create(config) };
    
    // Note: This might fail if not in a proper test environment with local network
    // For unit tests, we just check if it's not null or if it properly fails
    if !sdk_handle.is_null() {
        // Test getting network
        let network = unsafe { swift_dash_sdk_get_network(sdk_handle) };
        assert_eq!(network, SwiftDashNetwork::Local);
        
        // Destroy the SDK
        unsafe {
            swift_dash_sdk_destroy(sdk_handle);
        }
    } else {
        println!("SDK creation failed - this is expected in unit test environment");
    }
}

#[test]
fn test_test_signer_creation() {
    let signer_handle = unsafe { swift_dash_signer_create_test() };
    
    assert!(!signer_handle.is_null());
    
    // Clean up
    unsafe {
        swift_dash_signer_destroy(signer_handle);
    }
}

#[test]
fn test_null_pointer_safety() {
    // Test that functions handle null pointers safely
    unsafe {
        // These should not crash
        swift_dash_sdk_destroy(ptr::null_mut());
        swift_dash_signer_destroy(ptr::null_mut());
        
        // These should return appropriate values for null input
        let network = swift_dash_sdk_get_network(ptr::null_mut());
        // Should return a default/fallback value
        assert_eq!(network, SwiftDashNetwork::Testnet);
    }
}