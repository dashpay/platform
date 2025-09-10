//! Name registration operations

use std::os::raw::c_char;

use crate::types::{IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode};

/// Register a name for an identity
///
/// # Safety
/// - `_sdk_handle` and `_identity_handle` must be valid pointers when used; currently this stub ignores them.
/// - `_name` must be a valid pointer to a NUL-terminated C string if used in the future.
/// - Returns a heap-allocated error pointer; caller must free it using `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_register_name(
    _sdk_handle: *mut SDKHandle,
    _identity_handle: *const IdentityHandle,
    _name: *const c_char,
) -> *mut DashSDKError {
    // TODO: Implement name registration once the SDK API is available
    Box::into_raw(Box::new(DashSDKError::new(
        DashSDKErrorCode::NotImplemented,
        "Name registration not yet implemented".to_string(),
    )))
}
