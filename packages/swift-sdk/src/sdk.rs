// All imports removed as none are currently used
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

impl From<SwiftDashNetwork> for rs_sdk_ffi::DashSDKNetwork {
    fn from(network: SwiftDashNetwork) -> Self {
        match network {
            SwiftDashNetwork::Mainnet => rs_sdk_ffi::DashSDKNetwork::Mainnet,
            SwiftDashNetwork::Testnet => rs_sdk_ffi::DashSDKNetwork::Testnet,
            SwiftDashNetwork::Devnet => rs_sdk_ffi::DashSDKNetwork::Devnet,
            SwiftDashNetwork::Local => rs_sdk_ffi::DashSDKNetwork::Local,
        }
    }
}

/// Configuration for the Swift Dash Platform SDK
#[repr(C)]
pub struct SwiftDashSDKConfig {
    pub network: SwiftDashNetwork,
    pub dapi_addresses: *const c_char, // Comma-separated list of addresses
}

impl From<&SwiftDashSDKConfig> for rs_sdk_ffi::DashSDKConfig {
    fn from(config: &SwiftDashSDKConfig) -> Self {
        rs_sdk_ffi::DashSDKConfig {
            network: config.network.into(),
            dapi_addresses: config.dapi_addresses,
            skip_asset_lock_proof_verification: false,
            request_retry_count: 3,
            request_timeout_ms: 30000,
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

impl From<SwiftDashPutSettings> for rs_sdk_ffi::DashSDKPutSettings {
    fn from(settings: SwiftDashPutSettings) -> Self {
        rs_sdk_ffi::DashSDKPutSettings {
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
    let ffi_config = (&config).into();

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_create(&ffi_config);

        if !result.error.is_null() {
            // Clean up error and return null
            let error = Box::from_raw(result.error);
            drop(error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::SDKHandle
    }
}

/// Destroy an SDK instance
#[no_mangle]
pub unsafe extern "C" fn swift_dash_sdk_destroy(handle: *mut rs_sdk_ffi::SDKHandle) {
    if !handle.is_null() {
        rs_sdk_ffi::dash_sdk_destroy(handle);
    }
}

/// Get the network the SDK is configured for
#[no_mangle]
pub extern "C" fn swift_dash_sdk_get_network(
    handle: *const rs_sdk_ffi::SDKHandle,
) -> SwiftDashNetwork {
    unsafe {
        let network = rs_sdk_ffi::dash_sdk_get_network(handle);
        match network {
            rs_sdk_ffi::DashSDKNetwork::Mainnet => SwiftDashNetwork::Mainnet,
            rs_sdk_ffi::DashSDKNetwork::Testnet => SwiftDashNetwork::Testnet,
            rs_sdk_ffi::DashSDKNetwork::Devnet => SwiftDashNetwork::Devnet,
            rs_sdk_ffi::DashSDKNetwork::Local => SwiftDashNetwork::Local,
        }
    }
}

/// Get SDK version
#[no_mangle]
pub extern "C" fn swift_dash_sdk_get_version() -> *const c_char {
    rs_sdk_ffi::dash_sdk_version()
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
        dapi_addresses: ptr::null(),
    }
}

/// Create default config for testnet
#[no_mangle]
pub extern "C" fn swift_dash_sdk_config_testnet() -> SwiftDashSDKConfig {
    SwiftDashSDKConfig {
        network: SwiftDashNetwork::Testnet,
        dapi_addresses: ptr::null(),
    }
}

/// Create default config for local development
#[no_mangle]
pub extern "C" fn swift_dash_sdk_config_local() -> SwiftDashSDKConfig {
    SwiftDashSDKConfig {
        network: SwiftDashNetwork::Local,
        dapi_addresses: ptr::null(),
    }
}
