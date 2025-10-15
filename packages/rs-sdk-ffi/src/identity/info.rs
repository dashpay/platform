//! Identity information operations

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identity;
use std::ffi::CString;

use crate::types::{DashSDKIdentityInfo, IdentityHandle};

/// Get identity information
///
/// # Safety
/// - `identity_handle` must be a valid, non-null pointer to an `IdentityHandle` that remains valid for the duration of the call.
/// - Returns a heap-allocated `DashSDKIdentityInfo` pointer; caller must free it using the SDK-provided destroy function.
/// - Passing invalid or dangling pointers results in undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_get_info(
    identity_handle: *const IdentityHandle,
) -> *mut DashSDKIdentityInfo {
    if identity_handle.is_null() {
        return std::ptr::null_mut();
    }

    let identity = &*(identity_handle as *const Identity);

    let id_str = match CString::new(identity.id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };

    let info = DashSDKIdentityInfo {
        id: id_str,
        balance: identity.balance(),
        revision: identity.revision(),
        public_keys_count: identity.public_keys().len() as u32,
    };

    Box::into_raw(Box::new(info))
}

/// Destroy an identity handle
///
/// # Safety
/// - `handle` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `handle` becomes invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_destroy(handle: *mut IdentityHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Identity);
    }
}
