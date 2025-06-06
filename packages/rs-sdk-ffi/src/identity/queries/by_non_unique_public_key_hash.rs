//! Identity by non-unique public key hash query operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::platform::types::identity::NonUniquePublicKeyHashQuery;
use dash_sdk::platform::Fetch;
use dash_sdk::platform::{Identifier, Identity};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch identity by non-unique public key hash with optional pagination
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `public_key_hash`: Hex-encoded 20-byte public key hash
/// - `start_after`: Optional Base58-encoded identity ID to start after (for pagination)
///
/// # Returns
/// JSON string containing the identity information, or null if not found
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_fetch_by_non_unique_public_key_hash(
    sdk_handle: *const SDKHandle,
    public_key_hash: *const c_char,
    start_after: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || public_key_hash.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or public key hash is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let hash_str = match CStr::from_ptr(public_key_hash).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    // Parse hex-encoded public key hash
    let hash_bytes = match hex::decode(hash_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid hex-encoded public key hash: {}", e),
            ))
        }
    };

    if hash_bytes.len() != 20 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "Public key hash must be exactly 20 bytes, got {}",
                hash_bytes.len()
            ),
        ));
    }

    let mut key_hash = [0u8; 20];
    key_hash.copy_from_slice(&hash_bytes);

    // Parse optional start_after identity ID
    let after = if !start_after.is_null() {
        let after_str = match CStr::from_ptr(start_after).to_str() {
            Ok(s) => s,
            Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
        };

        match Identifier::from_string(after_str, Encoding::Base58) {
            Ok(id) => {
                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(id.as_bytes());
                Some(bytes)
            }
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid start_after identity ID: {}", e),
                ))
            }
        }
    } else {
        None
    };

    let result: Result<Option<Identity>, FFIError> = wrapper.runtime.block_on(async {
        // Fetch identity by non-unique public key hash
        let query = NonUniquePublicKeyHashQuery { key_hash, after };
        Identity::fetch(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)
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
