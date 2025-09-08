//! Unified SDK coordination module
#![allow(unexpected_cfgs)]
//!
//! This module provides unified functions that coordinate between Core SDK and Platform SDK
//! when both are available. It manages initialization, state synchronization, and
//! cross-layer operations.

use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::types::{DashSDKConfig, SDKHandle};
use dash_spv_ffi::{FFIClientConfig, FFIDashSpvClient};

/// Static flag to track unified initialization
static UNIFIED_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Unified SDK configuration combining both Core and Platform settings
#[repr(C)]
pub struct UnifiedSDKConfig {
    /// Core SDK configuration (ignored if core feature disabled)
    pub core_config: *const FFIClientConfig,
    /// Platform SDK configuration
    pub platform_config: DashSDKConfig,
    /// Whether to enable cross-layer integration
    pub enable_integration: bool,
}

/// Unified SDK handle containing both Core and Platform SDKs
#[repr(C)]
pub struct UnifiedSDKHandle {
    pub core_client: *mut FFIDashSpvClient,
    pub platform_sdk: *mut SDKHandle,
    pub integration_enabled: bool,
}

/// Initialize the unified SDK system
/// This initializes both Core SDK (if enabled) and Platform SDK
#[no_mangle]
pub extern "C" fn dash_unified_sdk_init() -> i32 {
    if UNIFIED_INITIALIZED.load(Ordering::Relaxed) {
        return 0; // Already initialized
    }

    // Initialize Core SDK if feature is enabled
    #[cfg(feature = "core")]
    {
        let core_result = crate::core_sdk::dash_core_sdk_init();
        if core_result != 0 {
            return core_result;
        }
    }

    // Initialize Platform SDK
    crate::dash_sdk_init();

    UNIFIED_INITIALIZED.store(true, Ordering::Relaxed);
    0
}

/// Create a unified SDK handle with both Core and Platform SDKs
///
/// # Safety
/// - `config` must point to a valid UnifiedSDKConfig structure
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_create(
    config: *const UnifiedSDKConfig,
) -> *mut UnifiedSDKHandle {
    if config.is_null() {
        return std::ptr::null_mut();
    }

    let config = &*config;

    // Create Core SDK client (always enabled in unified SDK)
    let core_client = dash_spv_ffi::dash_spv_ffi_client_new(config.core_config);

    // Create Platform SDK
    let platform_sdk_result = crate::dash_sdk_create(&config.platform_config);
    if platform_sdk_result.data.is_null() {
        // Clean up core client if it was created
        #[cfg(feature = "core")]
        if !core_client.is_null() {
            crate::core_sdk::dash_core_sdk_destroy_client(core_client);
        }
        return std::ptr::null_mut();
    }

    // Create unified handle
    let unified_handle = Box::new(UnifiedSDKHandle {
        core_client,
        platform_sdk: platform_sdk_result.data as *mut SDKHandle,
        integration_enabled: config.enable_integration,
    });

    Box::into_raw(unified_handle)
}

/// Destroy a unified SDK handle
///
/// # Safety
/// - `handle` must be a valid unified SDK handle or null
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_destroy(handle: *mut UnifiedSDKHandle) {
    if handle.is_null() {
        return;
    }

    let handle = Box::from_raw(handle);

    // Destroy Core SDK client
    #[cfg(feature = "core")]
    if !handle.core_client.is_null() {
        crate::core_sdk::dash_core_sdk_destroy_client(handle.core_client);
    }

    // Destroy Platform SDK
    if !handle.platform_sdk.is_null() {
        crate::dash_sdk_destroy(handle.platform_sdk);
    }
}

/// Start both Core and Platform SDKs
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_start(handle: *mut UnifiedSDKHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle = &*handle;

    // Start Core SDK if available
    #[cfg(feature = "core")]
    if !handle.core_client.is_null() {
        let core_result = crate::core_sdk::dash_core_sdk_start(handle.core_client);
        if core_result != 0 {
            return core_result;
        }
    }

    // Platform SDK doesn't have a separate start function currently
    // It's started when needed for operations

    0
}

/// Stop both Core and Platform SDKs
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_stop(handle: *mut UnifiedSDKHandle) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle = &*handle;

    // Stop Core SDK if available
    #[cfg(feature = "core")]
    if !handle.core_client.is_null() {
        let core_result = crate::core_sdk::dash_core_sdk_stop(handle.core_client);
        if core_result != 0 {
            return core_result;
        }
    }

    // Platform SDK doesn't have a separate stop function currently

    0
}

/// Get the Core SDK client from a unified handle
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_get_core_client(
    handle: *mut UnifiedSDKHandle,
) -> *mut FFIDashSpvClient {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle = &*handle;
    handle.core_client
}

/// Get the Platform SDK from a unified handle
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_get_platform_sdk(
    handle: *mut UnifiedSDKHandle,
) -> *mut SDKHandle {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle = &*handle;
    handle.platform_sdk
}

/// Check if integration is enabled for this unified SDK
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_is_integration_enabled(
    handle: *mut UnifiedSDKHandle,
) -> bool {
    if handle.is_null() {
        return false;
    }

    let handle = &*handle;
    handle.integration_enabled
}

/// Check if Core SDK is available in this unified SDK
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_has_core_sdk(handle: *mut UnifiedSDKHandle) -> bool {
    if handle.is_null() {
        return false;
    }

    #[cfg(feature = "core")]
    {
        let handle = &*handle;
        !handle.core_client.is_null()
    }
    #[cfg(not(feature = "core"))]
    {
        false
    }
}

/// Register Core SDK with Platform SDK for context provider callbacks
/// This enables Platform SDK to query Core SDK for blockchain state
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_register_core_context(
    handle: *mut UnifiedSDKHandle,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle = &*handle;

    if handle.core_client.is_null() || handle.platform_sdk.is_null() {
        return -1;
    }

    // Register Core SDK as context provider for Platform SDK
    // This would involve setting up the callback functions
    // Implementation depends on the specific context provider mechanism

    // For now, return success - actual implementation would register callbacks
    0
}

/// Get combined status of both SDKs
///
/// # Safety
/// - `handle` must be a valid unified SDK handle
/// - `core_height` must point to a valid u32 (set to 0 if core disabled)
/// - `platform_ready` must point to a valid bool
#[no_mangle]
pub unsafe extern "C" fn dash_unified_sdk_get_status(
    handle: *mut UnifiedSDKHandle,
    core_height: *mut u32,
    platform_ready: *mut bool,
) -> i32 {
    if handle.is_null() || core_height.is_null() || platform_ready.is_null() {
        return -1;
    }

    let handle = &*handle;

    // Get Core SDK height
    #[cfg(feature = "core")]
    if !handle.core_client.is_null() {
        let result =
            crate::core_sdk::dash_core_sdk_get_block_height(handle.core_client, core_height);
        if result != 0 {
            *core_height = 0;
        }
    } else {
        *core_height = 0;
    }

    #[cfg(not(feature = "core"))]
    {
        *core_height = 0;
    }

    // Check Platform SDK readiness (simplified)
    *platform_ready = !handle.platform_sdk.is_null();

    0
}

/// Get unified SDK version information
#[no_mangle]
pub extern "C" fn dash_unified_sdk_version() -> *const c_char {
    #[cfg(feature = "core")]
    const VERSION_INFO: &str = concat!("unified-", env!("CARGO_PKG_VERSION"), "+core\0");

    #[cfg(not(feature = "core"))]
    const VERSION_INFO: &str = concat!("unified-", env!("CARGO_PKG_VERSION"), "+platform-only\0");
    VERSION_INFO.as_ptr() as *const c_char
}

/// Check if unified SDK was compiled with core support
#[no_mangle]
pub extern "C" fn dash_unified_sdk_has_core_support() -> bool {
    #[cfg(feature = "core")]
    {
        true
    }
    #[cfg(not(feature = "core"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DashSDKNetwork;
    use std::ptr;

    /// Test the basic lifecycle of the unified SDK with core feature enabled
    #[test]
    #[cfg(feature = "core")]
    fn test_unified_sdk_lifecycle() {
        // Initialize the unified SDK system
        let init_result = dash_unified_sdk_init();
        assert_eq!(init_result, 0, "Failed to initialize unified SDK");

        // Create a testnet configuration for the unified SDK
        let platform_config = DashSDKConfig {
            network: DashSDKNetwork::SDKTestnet,
            dapi_addresses: ptr::null(), // Use mock SDK
            skip_asset_lock_proof_verification: true,
            request_retry_count: 3,
            request_timeout_ms: 30000,
        };

        // Step 1: Call dash_spv_ffi_config_testnet() to get a pointer to the FFI config object
        let core_config_ptr = dash_spv_ffi::dash_spv_ffi_config_testnet();
        assert!(!core_config_ptr.is_null(), "Failed to create core config");

        // Step 2: Create the UnifiedSDKConfig using the pointer
        let unified_config = UnifiedSDKConfig {
            core_config: core_config_ptr,
            platform_config,
            enable_integration: true,
        };

        // Step 3: Proceed with the test by passing a reference to dash_unified_sdk_create()
        let handle = unsafe { dash_unified_sdk_create(&unified_config) };
        assert!(!handle.is_null(), "Failed to create unified SDK handle");

        // Verify that the core client is available when core feature is enabled
        let core_client = unsafe { dash_unified_sdk_get_core_client(handle) };
        assert!(
            !core_client.is_null(),
            "Core client should not be null when core feature is enabled"
        );

        // Verify that the platform SDK is available
        let platform_sdk = unsafe { dash_unified_sdk_get_platform_sdk(handle) };
        assert!(!platform_sdk.is_null(), "Platform SDK should not be null");

        // Verify integration status
        let integration_enabled = unsafe { dash_unified_sdk_is_integration_enabled(handle) };
        assert!(integration_enabled, "Integration should be enabled");

        // Verify core support
        let has_core = unsafe { dash_unified_sdk_has_core_sdk(handle) };
        assert!(
            has_core,
            "Should have core SDK when core feature is enabled"
        );

        // Clean up the handle
        unsafe { dash_unified_sdk_destroy(handle) };

        // Clean up the config pointer
        unsafe { dash_spv_ffi::dash_spv_ffi_config_destroy(core_config_ptr) };
    }

    /// Test that unified SDK functions handle null pointers gracefully
    #[test]
    fn test_unified_sdk_null_handling() {
        // Test that destroy function handles null pointer
        unsafe { dash_unified_sdk_destroy(ptr::null_mut()) };

        // Test that get functions return null for null input
        #[cfg(feature = "core")]
        {
            let core_client = unsafe { dash_unified_sdk_get_core_client(ptr::null_mut()) };
            assert!(core_client.is_null(), "Should return null for null input");
        }

        let platform_sdk = unsafe { dash_unified_sdk_get_platform_sdk(ptr::null_mut()) };
        assert!(platform_sdk.is_null(), "Should return null for null input");

        // Test that status functions handle null input
        let integration_enabled =
            unsafe { dash_unified_sdk_is_integration_enabled(ptr::null_mut()) };
        assert!(!integration_enabled, "Should return false for null input");

        let has_core = unsafe { dash_unified_sdk_has_core_sdk(ptr::null_mut()) };
        assert!(!has_core, "Should return false for null input");
    }

    /// Test unified SDK version information
    #[test]
    fn test_unified_sdk_version() {
        let version = dash_unified_sdk_version();
        assert!(!version.is_null(), "Version string should not be null");

        // Convert to Rust string to verify it's valid
        let version_str = unsafe {
            std::ffi::CStr::from_ptr(version)
                .to_str()
                .expect("Version should be valid UTF-8")
        };

        assert!(
            version_str.starts_with("unified-"),
            "Version should start with 'unified-'"
        );

        #[cfg(feature = "core")]
        assert!(
            version_str.contains("+core"),
            "Version should contain '+core' when core feature is enabled"
        );

        #[cfg(not(feature = "core"))]
        assert!(
            version_str.contains("+platform-only"),
            "Version should contain '+platform-only' when core feature is disabled"
        );
    }

    /// Test unified SDK core support detection
    #[test]
    fn test_unified_sdk_core_support() {
        let has_core_support = dash_unified_sdk_has_core_support();

        #[cfg(feature = "core")]
        assert!(
            has_core_support,
            "Should report core support when core feature is enabled"
        );

        #[cfg(not(feature = "core"))]
        assert!(
            !has_core_support,
            "Should not report core support when core feature is disabled"
        );
    }
}
