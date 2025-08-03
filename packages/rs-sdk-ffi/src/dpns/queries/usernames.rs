//! Get DPNS usernames for an identity

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::identifier::Identifier;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use serde_json::json;

/// Get DPNS usernames owned by an identity
///
/// This function returns all DPNS usernames associated with a given identity ID.
/// It checks for domains where the identity is:
/// - The owner of the domain document
/// - Listed in records.dashUniqueIdentityId
/// - Listed in records.dashAliasIdentityId
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `identity_id` - The identity ID to search for (base58 string)
/// * `limit` - Maximum number of results to return (0 for default of 10)
///
/// # Returns
/// * On success: A JSON array of username objects
/// * On error: An error result
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_usernames(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    limit: u32,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let sdk_wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = &sdk_wrapper.sdk;

    // Convert identity ID from string
    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    let identifier = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    let limit_opt = if limit == 0 { None } else { Some(limit) };

    // Execute the async operation
    let result = sdk_wrapper.runtime.block_on(async {
        match sdk.get_dpns_usernames_by_identity(identifier, limit_opt).await {
            Ok(usernames) => {
                // Convert to JSON array
                let json_array: Vec<serde_json::Value> = usernames
                    .into_iter()
                    .map(|username| {
                        let mut obj = json!({
                            "label": username.label,
                            "normalizedLabel": username.normalized_label,
                            "fullName": username.full_name,
                            "ownerId": username.owner_id.to_string(Encoding::Base58)
                        });

                        if let Some(id) = username.records_identity_id {
                            obj["recordsIdentityId"] = json!(id.to_string(Encoding::Base58));
                        }

                        obj
                    })
                    .collect();

                Ok(json!(json_array).to_string())
            }
            Err(e) => Err(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to get DPNS usernames: {}", e),
            )),
        }
    });

    match result {
        Ok(json) => {
            match CString::new(json) {
                Ok(c_string) => DashSDKResult::success_string(c_string.into_raw()),
                Err(_) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    "Failed to convert JSON to C string".to_string(),
                )),
            }
        }
        Err(e) => DashSDKResult::error(e),
    }
}