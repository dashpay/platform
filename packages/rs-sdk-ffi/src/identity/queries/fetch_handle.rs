//! Identity fetch operations that return handles

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::platform::Fetch;
use std::ffi::CStr;
use std::os::raw::c_char;
use tracing::{debug, error, info, warn};

use crate::sdk::SDKWrapper;
use crate::types::{DashSDKResultDataType, IdentityHandle, SDKHandle};
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
    info!("dash_sdk_identity_fetch_handle: called");

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
            debug!(
                identity_id = s,
                "dash_sdk_identity_fetch_handle: identity id"
            );
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

    debug!("dash_sdk_identity_fetch_handle: fetching identity");
    let result = wrapper.runtime.block_on(async {
        Identity::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(identity)) => {
            debug!("dash_sdk_identity_fetch_handle: identity fetched");
            debug!(id = ?identity.id(), balance = identity.balance(), revision = identity.revision(), keys = identity.public_keys().len(), "dash_sdk_identity_fetch_handle: identity summary");

            // List all keys
            for (key_id, key) in identity.public_keys() {
                debug!(key_id, purpose = ?key.purpose(), key_type = ?key.key_type(), "dash_sdk_identity_fetch_handle: key");
            }

            // Verify we can find a transfer key
            let transfer_key = identity.get_first_public_key_matching(
                Purpose::TRANSFER,
                dash_sdk::dpp::identity::SecurityLevel::full_range().into(),
                dash_sdk::dpp::identity::KeyType::all_key_types().into(),
                true,
            );

            match transfer_key {
                Some(key) => debug!(
                    key_id = key.id(),
                    "dash_sdk_identity_fetch_handle: found transfer key"
                ),
                None => warn!("dash_sdk_identity_fetch_handle: no transfer key found"),
            }

            // Create handle from the fetched identity
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            debug!(ptr = ?handle, "dash_sdk_identity_fetch_handle: created handle");

            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Ok(None) => {
            error!("dash_sdk_identity_fetch_handle: identity not found");
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::NotFound,
                "Identity not found".to_string(),
            ))
        }
        Err(e) => {
            error!(error = ?e, "dash_sdk_identity_fetch_handle: error");
            DashSDKResult::error(e.into())
        }
    }
}
