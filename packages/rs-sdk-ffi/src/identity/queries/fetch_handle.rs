//! Identity fetch operations that return handles

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{Purpose, SecurityLevel, KeyType};
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
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Identity ID: {:?}", identity.id());
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Identity balance: {}", identity.balance());
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Identity revision: {}", identity.revision());
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Number of public keys: {}", identity.public_keys().len());
            
            // List all keys
            for (key_id, key) in identity.public_keys() {
                eprintln!("🔵 dash_sdk_identity_fetch_handle: Key {}: purpose={:?}, type={:?}", 
                    key_id, key.purpose(), key.key_type());
            }
            
            // Verify we can find a transfer key
            let transfer_key = identity.get_first_public_key_matching(
                Purpose::TRANSFER,
                dash_sdk::dpp::identity::SecurityLevel::full_range().into(),
                dash_sdk::dpp::identity::KeyType::all_key_types().into(),
                true,
            );
            
            match transfer_key {
                Some(key) => eprintln!("🔵 dash_sdk_identity_fetch_handle: Found transfer key with ID: {}", key.id()),
                None => eprintln!("⚠️ dash_sdk_identity_fetch_handle: No transfer key found!"),
            }
            
            // Create handle from the fetched identity
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            eprintln!("🔵 dash_sdk_identity_fetch_handle: Created handle at: {:p}", handle);
            
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