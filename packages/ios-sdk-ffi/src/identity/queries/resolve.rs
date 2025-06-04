//! Name resolution operations

use std::os::raw::c_char;

use crate::types::SDKHandle;
use crate::{IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Resolve a name to an identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_resolve_name(
    _sdk_handle: *const SDKHandle,
    _name: *const c_char,
) -> IOSSDKResult {
    // TODO: Implement name resolution once the SDK API is available
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Name resolution not yet implemented".to_string(),
    ))
}
