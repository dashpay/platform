//! Identity balance query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Fetch identity balance
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_id`: Base58-encoded identity ID
///
/// # Returns
/// The balance of the identity as a string
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_fetch_balance(
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

    let result: Result<IdentityBalance, FFIError> = wrapper.runtime.block_on(async {
        // Fetch identity balance using FetchUnproved trait
        let balance = IdentityBalance::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)?
            .ok_or_else(|| FFIError::InternalError("Identity balance not found".to_string()))?;

        Ok(balance)
    });

    match result {
        Ok(balance) => {
            let balance_str = match CString::new(balance.to_string()) {
                Ok(s) => s,
                Err(e) => {
                    return IOSSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            IOSSDKResult::success_string(balance_str.into_raw())
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
