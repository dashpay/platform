//! Identity fetch operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::platform::Fetch;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch an identity by ID
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_fetch(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> DashSDKResult {
    eprintln!("🔵 dash_sdk_identity_fetch: Called");

    if sdk_handle.is_null() {
        eprintln!("❌ dash_sdk_identity_fetch: SDK handle is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        eprintln!("❌ dash_sdk_identity_fetch: Identity ID is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    eprintln!("🔵 dash_sdk_identity_fetch: Got SDK wrapper");

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => {
            eprintln!("🔵 dash_sdk_identity_fetch: Identity ID string: '{}'", s);
            eprintln!("🔵 dash_sdk_identity_fetch: Identity ID length: {}", s.len());
            // Debug each character to find the problematic one
            for (i, ch) in s.chars().enumerate() {
                eprintln!("🔵 dash_sdk_identity_fetch: char[{}] = '{}' (U+{:04X})", i, ch, ch as u32);
            }
            s
        }
        Err(e) => {
            eprintln!(
                "❌ dash_sdk_identity_fetch: Failed to convert C string: {}",
                e
            );
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => {
            eprintln!("🔵 dash_sdk_identity_fetch: Parsed identifier successfully");
            id
        }
        Err(e) => {
            eprintln!(
                "❌ dash_sdk_identity_fetch: Failed to parse identity ID: {}",
                e
            );
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    eprintln!("🔵 dash_sdk_identity_fetch: About to fetch identity from network...");
    let result = wrapper.runtime.block_on(async {
        eprintln!("🔵 dash_sdk_identity_fetch: Inside async block");
        let fetch_result = Identity::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from);
        eprintln!(
            "🔵 dash_sdk_identity_fetch: Fetch completed with result: {:?}",
            fetch_result.is_ok()
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
