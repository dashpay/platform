//! Name resolution operations

use std::os::raw::c_char;

use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Resolve a name to an identity
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_resolve_name(
    _sdk_handle: *const SDKHandle,
    _name: *const c_char,
) -> DashSDKResult {
    // TODO: Implement name resolution once the SDK API is available
    DashSDKResult::error(DashSDKError::new(
        DashSDKErrorCode::NotImplemented,
        "Name resolution not yet implemented".to_string(),
    ))
}
