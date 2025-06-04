//! Identity public keys query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::{FetchMany, IdentityPublicKey};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Fetch identity public keys
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_id`: Base58-encoded identity ID
///
/// # Returns
/// A JSON string containing the identity's public keys
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_fetch_public_keys(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> IOSSDKResult {
    if sdk_handle.is_null() || identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle or identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    let result = wrapper.runtime.block_on(async {
        // Fetch identity public keys using FetchMany trait
        let public_keys = IdentityPublicKey::fetch_many(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)?;

        // Serialize to JSON
        serde_json::to_string(&public_keys)
            .map_err(|e| FFIError::InternalError(format!("Failed to serialize keys: {}", e)))
    });

    match result {
        Ok(json_str) => {
            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return IOSSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            IOSSDKResult::success_string(c_str.into_raw())
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
