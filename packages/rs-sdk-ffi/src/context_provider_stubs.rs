//! Stub implementations for Core SDK FFI functions
//!
//! These are temporary stubs for testing compilation.
//! In production, these symbols would be provided by linking against the Core SDK library.

use std::ffi::c_char;

// Local test-only definitions for stubs
#[repr(C)]
pub struct FFIResult {
    pub error_code: i32,
    pub error_message: *const c_char,
}

type FFIDashSpvClient = std::ffi::c_void;

// Only compile stubs for tests when explicitly enabled AND dash-spv FFI is not linked.
#[cfg(all(test, feature = "ffi_core_stubs", not(feature = "dash_spv")))]
#[no_mangle]
pub unsafe extern "C" fn ffi_dash_spv_get_quorum_public_key(
    _client: *mut FFIDashSpvClient,
    _quorum_type: u32,
    _quorum_hash: *const u8,
    _core_chain_locked_height: u32,
    out_pubkey: *mut u8,
) -> FFIResult {
    // Stub implementation - fill with test data
    if !out_pubkey.is_null() {
        let test_key = [0u8; 48];
        std::ptr::copy_nonoverlapping(test_key.as_ptr(), out_pubkey, 48);
    }

    FFIResult {
        error_code: 0,
        error_message: std::ptr::null(),
    }
}

#[cfg(all(test, feature = "ffi_core_stubs", not(feature = "dash_spv")))]
#[no_mangle]
pub unsafe extern "C" fn ffi_dash_spv_get_platform_activation_height(
    _client: *mut FFIDashSpvClient,
    out_height: *mut u32,
) -> FFIResult {
    // Stub implementation - return test height
    if !out_height.is_null() {
        *out_height = 1000000; // Example activation height
    }

    FFIResult {
        error_code: 0,
        error_message: std::ptr::null(),
    }
}

#[cfg(all(test, feature = "ffi_core_stubs", not(feature = "dash_spv")))]
#[no_mangle]
pub unsafe extern "C" fn ffi_dash_spv_get_core_handle(
    _client: *mut FFIDashSpvClient,
) -> *mut CoreSDKHandle {
    // Stub implementation
    std::ptr::null_mut()
}

#[cfg(all(test, feature = "ffi_core_stubs", not(feature = "dash_spv")))]
#[no_mangle]
pub unsafe extern "C" fn ffi_dash_spv_release_core_handle(_handle: *mut CoreSDKHandle) {
    // Stub implementation - nothing to do
}
