//! Identity fetch operations that return handles

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::platform::Fetch;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::{SDKHandle, IdentityHandle, DashSDKResultDataType};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch an identity by ID and return a handle
/// 
/// This function fetches an identity from the network and returns
/// a handle that can be used with other FFI functions like transfers.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_id`: Base58-encoded identity ID
///
/// # Returns
/// - Handle to the fetched identity on success
/// - Error if fetch fails or identity not found
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_fetch_handle(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> DashSDKResult {
    eprintln!("🔵 dash_sdk_identity_fetch_handle: Called");

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

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => {
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Identity ID: '{}'", s);
            s
        }
        Err(e) => {
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    eprintln!("🔵 dash_sdk_identity_fetch_handle: Fetching identity from network...");
    let result = wrapper.runtime.block_on(async {
        Identity::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(identity)) => {
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Identity fetched successfully");
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Identity balance: {}", identity.balance());
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Number of public keys: {}", identity.public_keys().len());
            
            // Create handle from the fetched identity
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Ok(None) => {
            eprintln!("❌ dash_sdk_identity_fetch_handle: Identity not found");
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::NotFound,
                "Identity not found".to_string(),
            ))
        }
        Err(e) => {
            eprintln!("❌ dash_sdk_identity_fetch_handle: Error: {:?}", e);
            DashSDKResult::error(e.into())
        }
    }
}