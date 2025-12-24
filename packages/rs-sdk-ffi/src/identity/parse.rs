//! Identity parsing operations

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::prelude::Identity;
use std::ffi::{c_char, CStr};

use crate::types::{DashSDKResultDataType, IdentityHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Parse an identity from JSON string to handle
///
/// This function takes a JSON string representation of an identity
/// (as returned by dash_sdk_identity_fetch) and converts it to an
/// identity handle that can be used with other FFI functions.
///
/// # Parameters
/// - `json_str`: JSON string containing the identity data
///
/// # Returns
/// - Handle to the parsed identity on success
/// - Error if JSON parsing fails
///
/// # Safety
/// - `json_str` must be a valid, non-null pointer to a NUL-terminated C string and remain valid for the duration of the call.
/// - On success, the returned `DashSDKResult` contains a heap-allocated handle which must be freed using the
///   appropriate SDK destroy function to avoid leaks.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_parse_json(json_str: *const c_char) -> DashSDKResult {
    if json_str.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "JSON string is null".to_string(),
        ));
    }

    let json = match CStr::from_ptr(json_str).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    eprintln!("🔵 dash_sdk_identity_parse_json: Parsing JSON: {}", json);

    match serde_json::from_str::<Identity>(json) {
        Ok(identity) => {
            eprintln!("🔵 dash_sdk_identity_parse_json: Successfully parsed identity");
            eprintln!(
                "🔵 dash_sdk_identity_parse_json: Identity ID: {:?}",
                identity.id()
            );
            eprintln!(
                "🔵 dash_sdk_identity_parse_json: Identity balance: {}",
                identity.balance()
            );
            eprintln!(
                "🔵 dash_sdk_identity_parse_json: Number of public keys: {}",
                identity.public_keys().len()
            );

            // Print public key details
            for (key_id, key) in identity.public_keys() {
                eprintln!(
                    "🔵 dash_sdk_identity_parse_json: Key {}: purpose={:?}, type={:?}",
                    key_id,
                    key.purpose(),
                    key.key_type()
                );
            }

            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            format!("Failed to parse identity JSON: {}", e),
        )),
    }
}
