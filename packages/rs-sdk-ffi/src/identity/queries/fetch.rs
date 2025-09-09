//! Identity fetch operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::platform::Fetch;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use tracing::{debug, error, info};

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch an identity by ID
///
/// # Safety
/// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
/// - `identity_id` must point to a NUL-terminated C string.
/// - On success, returns a handle or no data; any heap memory must be freed using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_fetch(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> DashSDKResult {
    info!("dash_sdk_identity_fetch: called");

    if sdk_handle.is_null() {
        error!("dash_sdk_identity_fetch: SDK handle is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        error!("dash_sdk_identity_fetch: identity ID is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    debug!("dash_sdk_identity_fetch: got SDK wrapper");

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => {
            debug!(
                identity_id = s,
                len = s.len(),
                "dash_sdk_identity_fetch: identity id"
            );
            s
        }
        Err(e) => {
            error!(error = %e, "dash_sdk_identity_fetch: failed to convert C string");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    // Try to parse as hex first (64 chars), then as Base58
    let id = if id_str.len() == 64 && id_str.chars().all(|c| c.is_ascii_hexdigit()) {
        debug!("dash_sdk_identity_fetch: detected hex format");
        match Identifier::from_string(id_str, Encoding::Hex) {
            Ok(id) => {
                debug!("dash_sdk_identity_fetch: parsed hex identifier");
                id
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_identity_fetch: failed to parse hex identity id");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid hex identity ID: {}", e),
                ));
            }
        }
    } else {
        debug!("dash_sdk_identity_fetch: trying Base58 format");
        match Identifier::from_string(id_str, Encoding::Base58) {
            Ok(id) => {
                debug!("dash_sdk_identity_fetch: parsed Base58 identifier");
                id
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_identity_fetch: failed to parse Base58 identity id");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!(
                        "Invalid identity ID. Must be either 64-char hex or valid Base58: {}",
                        e
                    ),
                ));
            }
        }
    };

    debug!("dash_sdk_identity_fetch: fetching identity");
    let result = wrapper.runtime.block_on(async {
        debug!("dash_sdk_identity_fetch: inside async block");
        let fetch_result = Identity::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from);
        debug!(
            ok = fetch_result.is_ok(),
            "dash_sdk_identity_fetch: fetch completed"
        );
        fetch_result
    });

    match result {
        Ok(Some(identity)) => {
            // Convert identity to JSON
            let json_str = match serde_json::to_string(&identity) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to serialize identity: {}", e))
                            .into(),
                    )
                }
            };

            let c_str = match CString::new(json_str) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(c_str.into_raw())
        }
        Ok(None) => {
            // Return null for not found
            DashSDKResult::success_string(std::ptr::null_mut())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
