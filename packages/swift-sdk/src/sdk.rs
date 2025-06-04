use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Network types for Dash Platform
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwiftDashNetwork {
    Mainnet = 0,
    Testnet = 1,
    Devnet = 2,
    Local = 3,
}

impl From<SwiftDashNetwork> for rs_sdk_ffi::IOSSDKNetwork {
    fn from(network: SwiftDashNetwork) -> Self {
        match network {
            SwiftDashNetwork::Mainnet => rs_sdk_ffi::IOSSDKNetwork::Mainnet,
            SwiftDashNetwork::Testnet => rs_sdk_ffi::IOSSDKNetwork::Testnet,
            SwiftDashNetwork::Devnet => rs_sdk_ffi::IOSSDKNetwork::Devnet,
            SwiftDashNetwork::Local => rs_sdk_ffi::IOSSDKNetwork::Local,
        }
    }
}

/// Configuration for the Swift Dash Platform SDK
#[repr(C)]
pub struct SwiftDashSDKConfig {
    pub network: SwiftDashNetwork,
    pub skip_asset_lock_proof_verification: bool,
    pub request_retry_count: u32,
    pub request_timeout_ms: u64,
}

impl From<SwiftDashSDKConfig> for rs_sdk_ffi::IOSSDKConfig {
    fn from(config: SwiftDashSDKConfig) -> Self {
        rs_sdk_ffi::IOSSDKConfig {
            network: config.network.into(),
            skip_asset_lock_proof_verification: config.skip_asset_lock_proof_verification,
            request_retry_count: config.request_retry_count,
            request_timeout_ms: config.request_timeout_ms,
        }
    }
}

/// Settings for put operations
#[repr(C)]
#[derive(Copy, Clone)]
pub struct SwiftDashPutSettings {
    pub connect_timeout_ms: u64,
    pub timeout_ms: u64,
    pub retries: u32,
    pub ban_failed_address: bool,
    pub identity_nonce_stale_time_s: u64,
    pub user_fee_increase: u16,
    pub allow_signing_with_any_security_level: bool,
    pub allow_signing_with_any_purpose: bool,
    pub wait_timeout_ms: u64,
}

impl From<SwiftDashPutSettings> for rs_sdk_ffi::IOSSDKPutSettings {
    fn from(settings: SwiftDashPutSettings) -> Self {
        rs_sdk_ffi::IOSSDKPutSettings {
            connect_timeout_ms: settings.connect_timeout_ms,
            timeout_ms: settings.timeout_ms,
            retries: settings.retries,
            ban_failed_address: settings.ban_failed_address,
            identity_nonce_stale_time_s: settings.identity_nonce_stale_time_s,
            user_fee_increase: settings.user_fee_increase,
            allow_signing_with_any_security_level: settings.allow_signing_with_any_security_level,
            allow_signing_with_any_purpose: settings.allow_signing_with_any_purpose,
            wait_timeout_ms: settings.wait_timeout_ms,
        }
    }
}

/// Create a new SDK instance
#[no_mangle]
pub extern "C" fn swift_dash_sdk_create(config: SwiftDashSDKConfig) -> *mut rs_sdk_ffi::SDKHandle {
    let ffi_config = config.into();

    unsafe {
        let result = rs_sdk_ffi::ios_sdk_create(&ffi_config);

        if !result.error.is_null() {
            // Clean up error and return null
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::SDKHandle
    }
}

/// Destroy an SDK instance
#[no_mangle]
pub unsafe extern "C" fn swift_dash_sdk_destroy(handle: *mut rs_sdk_ffi::SDKHandle) {
    if !handle.is_null() {
        rs_sdk_ffi::ios_sdk_destroy(handle);
    }
}

/// Get the network the SDK is configured for
#[no_mangle]
pub extern "C" fn swift_dash_sdk_get_network(
    handle: *mut rs_sdk_ffi::SDKHandle,
) -> SwiftDashNetwork {
    unsafe {
        let result = rs_sdk_ffi::ios_sdk_get_network(handle);

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return SwiftDashNetwork::Testnet; // Default fallback
        }

        let network_value = result.data as u32;
        match network_value {
            0 => SwiftDashNetwork::Mainnet,
            1 => SwiftDashNetwork::Testnet,
            2 => SwiftDashNetwork::Devnet,
            3 => SwiftDashNetwork::Local,
            _ => SwiftDashNetwork::Testnet, // Default fallback
        }
    }
}

/// Get SDK version
#[no_mangle]
pub extern "C" fn swift_dash_sdk_get_version() -> *mut c_char {
    unsafe {
        let result = rs_sdk_ffi::ios_sdk_version();

        if !result.error.is_null() {
            rs_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        // Make a copy of the version string that the caller can free
        let version_cstr = CStr::from_ptr(result.data as *const c_char);
        let version_string = CString::new(version_cstr.to_string_lossy().as_ref()).unwrap();

        // Free the original string
        rs_sdk_ffi::ios_sdk_string_free(result.data as *mut c_char);

        version_string.into_raw()
    }
}

/// Create default settings for put operations
#[no_mangle]
pub extern "C" fn swift_dash_put_settings_default() -> SwiftDashPutSettings {
    SwiftDashPutSettings {
        connect_timeout_ms: 0, // Use default
        timeout_ms: 0,         // Use default
        retries: 0,            // Use default
        ban_failed_address: false,
        identity_nonce_stale_time_s: 0, // Use default
        user_fee_increase: 0,
        allow_signing_with_any_security_level: false,
        allow_signing_with_any_purpose: false,
        wait_timeout_ms: 0, // Use default
    }
}

/// Create default config for mainnet
#[no_mangle]
pub extern "C" fn swift_dash_sdk_config_mainnet() -> SwiftDashSDKConfig {
    SwiftDashSDKConfig {
        network: SwiftDashNetwork::Mainnet,
        skip_asset_lock_proof_verification: false,
        request_retry_count: 3,
        request_timeout_ms: 30000,
    }
}

/// Create default config for testnet
#[no_mangle]
pub extern "C" fn swift_dash_sdk_config_testnet() -> SwiftDashSDKConfig {
    SwiftDashSDKConfig {
        network: SwiftDashNetwork::Testnet,
        skip_asset_lock_proof_verification: false,
        request_retry_count: 3,
        request_timeout_ms: 30000,
    }
}

/// Create default config for local development
#[no_mangle]
pub extern "C" fn swift_dash_sdk_config_local() -> SwiftDashSDKConfig {
    SwiftDashSDKConfig {
        network: SwiftDashNetwork::Local,
        skip_asset_lock_proof_verification: true,
        request_retry_count: 1,
        request_timeout_ms: 10000,
    }
}
