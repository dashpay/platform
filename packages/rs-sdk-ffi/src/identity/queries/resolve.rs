//! Name resolution operations

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Resolve a name to an identity
///
/// This function takes a name in the format "label.parentdomain" (e.g., "alice.dash")
/// or just "label" for top-level domains, and returns the associated identity ID.
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `name` - C string containing the name to resolve
///
/// # Returns
/// * On success: A result containing the resolved identity ID
/// * On error: An error result
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_resolve_name(
    sdk_handle: *const SDKHandle,
    name: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if name.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Name is null".to_string(),
        ));
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid UTF-8 in name".to_string(),
            ));
        }
    };

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    // Execute the async operation using the SDK's resolve_dpns_name method
    let result = sdk_wrapper.runtime.block_on(async {
        match sdk.resolve_dpns_name(name_str).await {
            Ok(Some(identity_id)) => Ok(identity_id.to_vec()),
            Ok(None) => Err(DashSDKError::new(
                DashSDKErrorCode::NotFound,
                format!("Name '{}' not found", name_str),
            )),
            Err(e) => Err(DashSDKError::new(
                DashSDKErrorCode::NetworkError,
                format!("Failed to resolve name: {}", e),
            )),
        }
    });

    match result {
        Ok(identity_id) => DashSDKResult::success_binary(identity_id),
        Err(e) => DashSDKResult::error(e),
    }
}
