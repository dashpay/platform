//! Core SDK FFI bindings
//!
//! This module provides FFI bindings for the Core SDK (SPV functionality).
//! It exposes Core SDK functions under the `dash_core_*` namespace to keep them 
//! separate from Platform SDK functions in the unified SDK.

use dash_spv_ffi::*;
use std::ffi::{c_char, CStr};
use crate::{DashSDKError, DashSDKErrorCode, FFIError};

/// Core SDK configuration structure (re-export from dash-spv-ffi)
pub use dash_spv_ffi::FFIClientConfig as CoreSDKConfig;

/// Core SDK client handle (re-export from dash-spv-ffi)
pub use dash_spv_ffi::FFIDashSpvClient as CoreSDKClient;

/// Initialize the Core SDK
/// Returns 0 on success, error code on failure
#[no_mangle]
pub extern "C" fn dash_core_sdk_init() -> i32 {
    // Core SDK initialization happens during client creation
    // This is a no-op for compatibility
    0
}

/// Create a Core SDK client with testnet config
/// 
/// # Safety
/// - Returns null on failure
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_create_client_testnet() -> *mut CoreSDKClient {
    // Create testnet configuration
    let config = dash_spv_ffi::dash_spv_ffi_config_testnet();
    if config.is_null() {
        return std::ptr::null_mut();
    }

    // Create the actual SPV client
    let client = dash_spv_ffi::dash_spv_ffi_client_new(config);
    
    // Clean up the config
    dash_spv_ffi::dash_spv_ffi_config_destroy(config);
    
    client as *mut CoreSDKClient
}

/// Create a Core SDK client with mainnet config
/// 
/// # Safety
/// - Returns null on failure
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_create_client_mainnet() -> *mut CoreSDKClient {
    // Create mainnet configuration
    let config = dash_spv_ffi::dash_spv_ffi_config_new(dash_spv_ffi::FFINetwork::Dash);
    if config.is_null() {
        return std::ptr::null_mut();
    }

    // Create the actual SPV client
    let client = dash_spv_ffi::dash_spv_ffi_client_new(config);
    
    // Clean up the config
    dash_spv_ffi::dash_spv_ffi_config_destroy(config);
    
    client as *mut CoreSDKClient
}

/// Create a Core SDK client with custom config
/// 
/// # Safety
/// - `config` must be a valid CoreSDKConfig pointer
/// - Returns null on failure
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_create_client(
    config: *const CoreSDKConfig,
) -> *mut CoreSDKClient {
    if config.is_null() {
        return std::ptr::null_mut();
    }

    // Create the actual SPV client using the provided config
    let client = dash_spv_ffi::dash_spv_ffi_client_new(config as *const dash_spv_ffi::FFIClientConfig);
    client as *mut CoreSDKClient
}

/// Destroy a Core SDK client
///
/// # Safety
/// - `client` must be a valid Core SDK client handle or null
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_destroy_client(client: *mut CoreSDKClient) {
    if !client.is_null() {
        dash_spv_ffi::dash_spv_ffi_client_destroy(client as *mut dash_spv_ffi::FFIDashSpvClient);
    }
}

/// Start the Core SDK client (begin sync)
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_start(client: *mut CoreSDKClient) -> i32 {
    if client.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_start(client as *mut dash_spv_ffi::FFIDashSpvClient)
}

/// Stop the Core SDK client
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_stop(client: *mut CoreSDKClient) -> i32 {
    if client.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_stop(client as *mut dash_spv_ffi::FFIDashSpvClient)
}

/// Sync Core SDK client to tip
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_sync_to_tip(client: *mut CoreSDKClient) -> i32 {
    if client.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_sync_to_tip(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
        None, // completion_callback
        std::ptr::null_mut(), // user_data
    )
}

/// Get the current sync progress
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - Returns pointer to FFISyncProgress structure (caller must free it)
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_sync_progress(
    client: *mut CoreSDKClient,
) -> *mut dash_spv_ffi::FFISyncProgress {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::dash_spv_ffi_client_get_sync_progress(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
    )
}

/// Get Core SDK statistics
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - Returns pointer to FFISpvStats structure (caller must free it)
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_stats(
    client: *mut CoreSDKClient,
) -> *mut dash_spv_ffi::FFISpvStats {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::dash_spv_ffi_client_get_stats(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
    )
}

/// Get the current block height
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `height` must point to a valid u32
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_block_height(
    client: *mut CoreSDKClient,
    height: *mut u32,
) -> i32 {
    if client.is_null() || height.is_null() {
        return -1;
    }

    // Get stats and extract block height from sync progress
    let stats = dash_spv_ffi::dash_spv_ffi_client_get_stats(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
    );
    
    if stats.is_null() {
        return -1;
    }
    
    *height = (*stats).header_height;
    
    // Clean up the stats pointer
    dash_spv_ffi::dash_spv_ffi_spv_stats_destroy(stats);
    0
}

/// Add an address to watch
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `address` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_watch_address(
    client: *mut CoreSDKClient,
    address: *const c_char,
) -> i32 {
    if client.is_null() || address.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_watch_address(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
        address,
    )
}

/// Remove an address from watching
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `address` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_unwatch_address(
    client: *mut CoreSDKClient,
    address: *const c_char,
) -> i32 {
    if client.is_null() || address.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_unwatch_address(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
        address,
    )
}

/// Get balance for all watched addresses
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - Returns pointer to FFIBalance structure (caller must free it)
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_total_balance(
    client: *mut CoreSDKClient,
) -> *mut dash_spv_ffi::FFIBalance {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::dash_spv_ffi_client_get_total_balance(
        client as *mut dash_spv_ffi::FFIDashSpvClient
    )
}

/// Get platform activation height
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `height` must point to a valid u32
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_platform_activation_height(
    client: *mut CoreSDKClient,
    height: *mut u32,
) -> i32 {
    if client.is_null() || height.is_null() {
        return -1;
    }

    let result = dash_spv_ffi::ffi_dash_spv_get_platform_activation_height(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
        height,
    );

    // FFIResult has an error_code field
    result.error_code
}

/// Get quorum public key
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `quorum_hash` must point to a valid 32-byte buffer
/// - `public_key` must point to a valid 48-byte buffer
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_quorum_public_key(
    client: *mut CoreSDKClient,
    quorum_type: u32,
    quorum_hash: *const u8,
    core_chain_locked_height: u32,
    public_key: *mut u8,
    public_key_size: usize,
) -> i32 {
    if client.is_null() || quorum_hash.is_null() || public_key.is_null() {
        return -1;
    }

    let result = dash_spv_ffi::ffi_dash_spv_get_quorum_public_key(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
        quorum_type,
        quorum_hash,
        core_chain_locked_height,
        public_key,
        public_key_size,
    );

    // FFIResult has an error_code field
    result.error_code
}

/// Get Core SDK handle for platform integration
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_core_handle(
    client: *mut CoreSDKClient,
) -> *mut dash_spv_ffi::CoreSDKHandle {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::ffi_dash_spv_get_core_handle(client as *mut dash_spv_ffi::FFIDashSpvClient)
}

/// Broadcast a transaction
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `transaction_hex` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_broadcast_transaction(
    client: *mut CoreSDKClient,
    transaction_hex: *const c_char,
) -> i32 {
    if client.is_null() || transaction_hex.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_broadcast_transaction(
        client as *mut dash_spv_ffi::FFIDashSpvClient,
        transaction_hex,
    )
}

/// Check if Core SDK feature is enabled at runtime
#[no_mangle]
pub extern "C" fn dash_core_sdk_is_enabled() -> bool {
    true // Always enabled in unified SDK
}

/// Get Core SDK version
#[no_mangle]
pub extern "C" fn dash_core_sdk_version() -> *const c_char {
    dash_spv_ffi::dash_spv_ffi_version()
}


