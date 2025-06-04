//! Name registration operations

use std::os::raw::c_char;

use crate::types::{IdentityHandle, SDKHandle};
use crate::{IOSSDKError, IOSSDKErrorCode};

/// Register a name for an identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_register_name(
    _sdk_handle: *mut SDKHandle,
    _identity_handle: *const IdentityHandle,
    _name: *const c_char,
) -> *mut IOSSDKError {
    // TODO: Implement name registration once the SDK API is available
    Box::into_raw(Box::new(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Name registration not yet implemented".to_string(),
    )))
}
