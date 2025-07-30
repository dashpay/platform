//! Core SDK FFI bindings
//!
//! This module provides FFI bindings for the Core SDK (SPV functionality).
//! It exposes Core SDK functions under the `dash_core_*` namespace to keep them 
//! separate from Platform SDK functions in the unified SDK.

use dash_spv_ffi::*;
use std::ffi::{c_char, CStr};
use crate::{DashSDKError, DashSDKErrorCode, FFIError};

// Note: We use FFIClientConfig and FFIDashSpvClient directly instead of type aliases
// to avoid C header generation issues with cbindgen

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
pub unsafe extern "C" fn dash_core_sdk_create_client_testnet() -> *mut FFIDashSpvClient {
    // Create testnet configuration
    let config = dash_spv_ffi::dash_spv_ffi_config_testnet();
    if config.is_null() {
        return std::ptr::null_mut();
    }

    // Create the actual SPV client
    let client = dash_spv_ffi::dash_spv_ffi_client_new(config);
    
    // Clean up the config
    dash_spv_ffi::dash_spv_ffi_config_destroy(config);
    
    client
}

/// Create a Core SDK client with mainnet config
/// 
/// # Safety
/// - Returns null on failure
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_create_client_mainnet() -> *mut FFIDashSpvClient {
    // Create mainnet configuration
    let config = dash_spv_ffi::dash_spv_ffi_config_new(dash_spv_ffi::FFINetwork::Dash);
    if config.is_null() {
        return std::ptr::null_mut();
    }

    // Create the actual SPV client
    let client = dash_spv_ffi::dash_spv_ffi_client_new(config);
    
    // Clean up the config
    dash_spv_ffi::dash_spv_ffi_config_destroy(config);
    
    client
}

/// Create a Core SDK client with custom config
/// 
/// # Safety
/// - `config` must be a valid CoreSDKConfig pointer
/// - Returns null on failure
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_create_client(
    config: *const FFIClientConfig,
) -> *mut FFIDashSpvClient {
    if config.is_null() {
        return std::ptr::null_mut();
    }

    // Create the actual SPV client using the provided config
    let client = dash_spv_ffi::dash_spv_ffi_client_new(config);
    client
}

/// Destroy a Core SDK client
///
/// # Safety
/// - `client` must be a valid Core SDK client handle or null
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_destroy_client(client: *mut FFIDashSpvClient) {
    if !client.is_null() {
        dash_spv_ffi::dash_spv_ffi_client_destroy(client);
    }
}

/// Start the Core SDK client (begin sync)
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_start(client: *mut FFIDashSpvClient) -> i32 {
    if client.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_start(client)
}

/// Stop the Core SDK client
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_stop(client: *mut FFIDashSpvClient) -> i32 {
    if client.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_stop(client)
}

/// Sync Core SDK client to tip
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_sync_to_tip(client: *mut FFIDashSpvClient) -> i32 {
    if client.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_sync_to_tip(
        client,
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
    client: *mut FFIDashSpvClient,
) -> *mut dash_spv_ffi::FFISyncProgress {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::dash_spv_ffi_client_get_sync_progress(
        client,
    )
}

/// Get Core SDK statistics
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - Returns pointer to FFISpvStats structure (caller must free it)
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_stats(
    client: *mut FFIDashSpvClient,
) -> *mut dash_spv_ffi::FFISpvStats {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::dash_spv_ffi_client_get_stats(
        client,
    )
}

/// Get the current block height
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `height` must point to a valid u32
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_block_height(
    client: *mut FFIDashSpvClient,
    height: *mut u32,
) -> i32 {
    if client.is_null() || height.is_null() {
        return -1;
    }

    // Get stats and extract block height from sync progress
    let stats = dash_spv_ffi::dash_spv_ffi_client_get_stats(
        client,
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
    client: *mut FFIDashSpvClient,
    address: *const c_char,
) -> i32 {
    if client.is_null() || address.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_watch_address(
        client,
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
    client: *mut FFIDashSpvClient,
    address: *const c_char,
) -> i32 {
    if client.is_null() || address.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_unwatch_address(
        client,
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
    client: *mut FFIDashSpvClient,
) -> *mut dash_spv_ffi::FFIBalance {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::dash_spv_ffi_client_get_total_balance(
        client
    )
}

/// Get platform activation height
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `height` must point to a valid u32
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_get_platform_activation_height(
    client: *mut FFIDashSpvClient,
    height: *mut u32,
) -> i32 {
    if client.is_null() || height.is_null() {
        return -1;
    }

    let result = dash_spv_ffi::ffi_dash_spv_get_platform_activation_height(
        client,
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
    client: *mut FFIDashSpvClient,
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
        client,
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
    client: *mut FFIDashSpvClient,
) -> *mut std::ffi::c_void {
    if client.is_null() {
        return std::ptr::null_mut();
    }

    dash_spv_ffi::ffi_dash_spv_get_core_handle(client) as *mut std::ffi::c_void
}

/// Broadcast a transaction
///
/// # Safety
/// - `client` must be a valid Core SDK client handle
/// - `transaction_hex` must be a valid null-terminated C string
#[no_mangle]
pub unsafe extern "C" fn dash_core_sdk_broadcast_transaction(
    client: *mut FFIDashSpvClient,
    transaction_hex: *const c_char,
) -> i32 {
    if client.is_null() || transaction_hex.is_null() {
        return -1;
    }

    dash_spv_ffi::dash_spv_ffi_client_broadcast_transaction(
        client,
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


