//! Identity nonce query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityNonceFetcher;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch identity nonce
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_id`: Base58-encoded identity ID
///
/// # Returns
/// The nonce of the identity as a string
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_fetch_nonce(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    let result: Result<u64, FFIError> = wrapper.runtime.block_on(async {
        // Fetch identity nonce
        let nonce_fetcher = IdentityNonceFetcher::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)?
            .ok_or_else(|| FFIError::InternalError("Identity nonce not found".to_string()))?;

        Ok(nonce_fetcher.0)
    });

    match result {
        Ok(nonce) => {
            let nonce_str = match CString::new(nonce.to_string()) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(nonce_str.into_raw())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
