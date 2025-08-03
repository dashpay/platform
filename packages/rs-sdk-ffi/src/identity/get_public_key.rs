//! Get public key from identity by key ID

use crate::types::{DashSDKPublicKeyHandle, IdentityHandle, DashSDKResultDataType};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use std::ptr;

/// Get a public key from an identity by its ID
///
/// # Parameters
/// - `identity`: Handle to the identity
/// - `key_id`: The ID of the public key to retrieve
///
/// # Returns
/// - Handle to the public key on success
/// - Error if key not found or invalid parameters
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_get_public_key_by_id(
    identity: *const IdentityHandle,
    key_id: u8,
) -> DashSDKResult {
    if identity.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity handle is null".to_string(),
        ));
    }

    let identity = &*(identity as *const dash_sdk::dpp::prelude::Identity);
    
    match identity.get_public_key_by_id(key_id.into()) {
        Some(public_key) => {
            let handle = Box::into_raw(Box::new(public_key.clone())) as *mut DashSDKPublicKeyHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultPublicKeyHandle,
            )
        }
        None => {
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Public key with ID {} not found in identity", key_id),
            ))
        }
    }
}

// Note: Public key destruction is handled by dash_sdk_identity_public_key_destroy in keys.rs