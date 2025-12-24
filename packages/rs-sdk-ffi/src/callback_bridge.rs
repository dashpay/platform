//! Callback bridge module for Core SDK integration
//!
//! This module implements the callback bridge pattern from dash-unified-ffi-old
//! to eliminate circular dependencies between Platform SDK and Core SDK.
//! Instead of direct linking, Core SDK functions are registered as callbacks
//! at runtime with the Platform SDK.

use crate::context_callbacks::{CallbackResult, ContextProviderCallbacks};
use std::ffi::c_void;

/// Register Core SDK handle and setup callback bridge with Platform SDK
///
/// This function implements the core pattern from dash-unified-ffi-old:
/// 1. Takes a Core SDK handle
/// 2. Creates callback wrappers for the functions Platform SDK needs
/// 3. Registers these callbacks with Platform SDK's context provider system
///
/// # Safety
/// - `core_handle` must be a valid Core SDK handle that remains valid for the SDK lifetime
/// - This function should be called once after creating both Core and Platform SDK instances
#[no_mangle]
pub unsafe extern "C" fn dash_unified_register_core_sdk_handle(core_handle: *mut c_void) -> i32 {
    if core_handle.is_null() {
        return -1;
    }

    // Create the callback structure with Core SDK function wrappers
    let callbacks = ContextProviderCallbacks {
        core_handle,
        get_platform_activation_height: bridge_get_platform_activation_height,
        get_quorum_public_key: bridge_get_quorum_public_key,
    };

    // Register the callbacks with Platform SDK's context provider system
    match crate::context_callbacks::set_global_callbacks(callbacks) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Bridge wrapper for Core SDK's get_platform_activation_height function
///
/// This function wraps the actual Core SDK function call in a callback-compatible signature.
/// It eliminates the circular dependency by calling the Core SDK function via extern declaration
/// rather than direct linking.
///
/// # Safety
/// - `handle` must be a valid Core SDK handle
/// - `out_height` must be a valid pointer to u32
unsafe extern "C" fn bridge_get_platform_activation_height(
    handle: *mut c_void,
    out_height: *mut u32,
) -> CallbackResult {
    if handle.is_null() || out_height.is_null() {
        return CallbackResult {
            success: false,
            error_code: -1,
            error_message: c"Invalid handle or output pointer".as_ptr(),
        };
    }

    // Call the actual Core SDK function via extern declaration
    // This avoids circular dependency while still accessing Core SDK functionality
    extern "C" {
        fn ffi_dash_spv_get_platform_activation_height(
            handle: *mut c_void,
            out_height: *mut u32,
        ) -> i32;
    }

    let result = ffi_dash_spv_get_platform_activation_height(handle, out_height);

    if result == 0 {
        CallbackResult {
            success: true,
            error_code: 0,
            error_message: std::ptr::null(),
        }
    } else {
        CallbackResult {
            success: false,
            error_code: result,
            error_message: c"Failed to get platform activation height".as_ptr(),
        }
    }
}

/// Bridge wrapper for Core SDK's get_quorum_public_key function
///
/// This function wraps the actual Core SDK function call in a callback-compatible signature.
///
/// # Safety
/// - `handle` must be a valid Core SDK handle
/// - `quorum_hash` must point to a valid 32-byte buffer
/// - `out_pubkey` must point to a valid 48-byte buffer
unsafe extern "C" fn bridge_get_quorum_public_key(
    handle: *mut c_void,
    quorum_type: u32,
    quorum_hash: *const u8,
    core_chain_locked_height: u32,
    out_pubkey: *mut u8,
) -> CallbackResult {
    if handle.is_null() || quorum_hash.is_null() || out_pubkey.is_null() {
        return CallbackResult {
            success: false,
            error_code: -1,
            error_message: c"Invalid handle or pointer parameters".as_ptr(),
        };
    }

    // Call the actual Core SDK function via extern declaration
    extern "C" {
        fn ffi_dash_spv_get_quorum_public_key(
            handle: *mut c_void,
            quorum_type: u32,
            quorum_hash: *const u8,
            core_chain_locked_height: u32,
            out_pubkey: *mut u8,
            pubkey_size: usize,
        ) -> i32;
    }

    let result = ffi_dash_spv_get_quorum_public_key(
        handle,
        quorum_type,
        quorum_hash,
        core_chain_locked_height,
        out_pubkey,
        48, // BLS public key size
    );

    if result == 0 {
        CallbackResult {
            success: true,
            error_code: 0,
            error_message: std::ptr::null(),
        }
    } else {
        CallbackResult {
            success: false,
            error_code: result,
            error_message: c"Failed to get quorum public key".as_ptr(),
        }
    }
}

/// Initialize the unified SDK system with callback bridge support
///
/// This function initializes both Core SDK and Platform SDK and sets up
/// the callback bridge pattern for inter-SDK communication.
#[no_mangle]
pub extern "C" fn dash_unified_init() -> i32 {
    // Initialize Platform SDK first
    crate::dash_sdk_init();

    // Note: Core SDK will be initialized when the client is created
    // The callback bridge will be set up when dash_unified_register_core_sdk_handle is called

    0
}

/// Get unified SDK version information including both Core and Platform components
#[no_mangle]
pub extern "C" fn dash_unified_version() -> *const std::os::raw::c_char {
    static VERSION: &str = concat!("unified-", env!("CARGO_PKG_VERSION"), "+core+platform\0");
    VERSION.as_ptr() as *const std::os::raw::c_char
}

/// Check if unified SDK has both Core and Platform support
#[no_mangle]
pub extern "C" fn dash_unified_has_full_support() -> bool {
    true // Always true in the unified approach
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_callback_bridge_null_handling() {
        // Test that bridge functions handle null pointers gracefully
        unsafe {
            let result = bridge_get_platform_activation_height(ptr::null_mut(), ptr::null_mut());
            assert!(!result.success);
            assert_eq!(result.error_code, -1);
        }
    }

    #[test]
    fn test_unified_init() {
        let result = dash_unified_init();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_unified_version() {
        let version = dash_unified_version();
        assert!(!version.is_null());

        let version_str = unsafe {
            std::ffi::CStr::from_ptr(version)
                .to_str()
                .expect("Version should be valid UTF-8")
        };

        assert!(version_str.starts_with("unified-"));
        assert!(version_str.contains("+core+platform"));
    }

    #[test]
    fn test_unified_support() {
        assert!(dash_unified_has_full_support());
    }
}
