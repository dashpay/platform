//! Identity operations

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::Fetch;
use dpp::dashcore::{Network, PrivateKey};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{AssetLockProof, Identifier, Identity};
use platform_value::string_encoding::Encoding;

use crate::sdk::SDKWrapper;
use crate::types::{IOSSDKIdentityInfo, IdentityHandle, SDKHandle};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Fetch an identity by ID
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_fetch(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> IOSSDKResult {
    if sdk_handle.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_id.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => {
            return IOSSDKResult::error(FFIError::from(e).into());
        }
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    let result = wrapper.runtime.block_on(async {
        Identity::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(identity)) => {
            let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::NotFound,
            "Identity not found".to_string(),
        )),
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Create a new identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_create(sdk_handle: *mut SDKHandle) -> IOSSDKResult {
    if sdk_handle.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    // TODO: Implement identity creation once the SDK API is available
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Identity creation not yet implemented".to_string(),
    ))
}

/// Top up an identity with credits
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_topup(
    _sdk_handle: *mut SDKHandle,
    _identity_handle: *const IdentityHandle,
    _amount: u64,
) -> *mut IOSSDKError {
    // TODO: Implement identity top-up once the SDK API is available
    Box::into_raw(Box::new(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Identity top-up not yet implemented".to_string(),
    )))
}

/// Get identity information
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_get_info(
    identity_handle: *const IdentityHandle,
) -> *mut IOSSDKIdentityInfo {
    if identity_handle.is_null() {
        return std::ptr::null_mut();
    }

    let identity = &*(identity_handle as *const Identity);

    let id_str = match CString::new(identity.id().to_string(Encoding::Base58)) {
        Ok(s) => s.into_raw(),
        Err(_) => return std::ptr::null_mut(),
    };

    let info = IOSSDKIdentityInfo {
        id: id_str,
        balance: identity.balance(),
        revision: identity.revision() as u64,
        public_keys_count: identity.public_keys().len() as u32,
    };

    Box::into_raw(Box::new(info))
}

/// Destroy an identity handle
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_destroy(handle: *mut IdentityHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut Identity);
    }
}

/// Register a name for an identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_register_name(
    _sdk_handle: *mut SDKHandle,
    _identity_handle: *const IdentityHandle,
    _name: *const c_char,
) -> *mut IOSSDKError {
    // TODO: Implement name registration once the SDK API is available
    Box::into_raw(Box::new(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Name registration not yet implemented".to_string(),
    )))
}

/// Resolve a name to an identity
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_resolve_name(
    _sdk_handle: *const SDKHandle,
    _name: *const c_char,
) -> IOSSDKResult {
    // TODO: Implement name resolution once the SDK API is available
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Name resolution not yet implemented".to_string(),
    ))
}

/// Put identity to platform (broadcast state transition)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_put_to_platform(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    asset_lock_proof_type: u8, // 0 = instant, 1 = chain
    asset_lock_proof_data: *const u8,
    asset_lock_proof_data_len: usize,
    asset_lock_proof_private_key: *const [u8; 32], // Private key as bytes
    signer_handle: *const crate::types::SignerHandle,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || asset_lock_proof_data.is_null()
        || asset_lock_proof_private_key.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let private_key_bytes = *asset_lock_proof_private_key;

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Convert private key bytes to PrivateKey
        let secp = dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key =
            dpp::dashcore::secp256k1::SecretKey::from_byte_array(&private_key_bytes)
                .map_err(|e| FFIError::InternalError(format!("Invalid private key: {}", e)))?;
        let private_key = PrivateKey::new(secret_key, Network::Dash);

        // Parse asset lock proof data
        let proof_data =
            std::slice::from_raw_parts(asset_lock_proof_data, asset_lock_proof_data_len);

        // For now, create a simple instant asset lock proof as a placeholder
        // In a real implementation, you would parse the proof_data based on asset_lock_proof_type
        let asset_lock_proof = if asset_lock_proof_type == 0 {
            // Instant asset lock proof
            // This is a placeholder - real implementation would deserialize from proof_data
            return Err(FFIError::InternalError(
                "Instant asset lock proof parsing not implemented".to_string(),
            ));
        } else {
            // Chain asset lock proof
            // This is a placeholder - real implementation would deserialize from proof_data
            return Err(FFIError::InternalError(
                "Chain asset lock proof parsing not implemented".to_string(),
            ));
        };

        // Use PutIdentity trait to put identity to platform
        let _state_transition = identity
            .put_to_platform(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put identity to platform: {}", e))
            })?;

        // For now, just return success. In a full implementation, you would return the state transition ID
        Ok("success".to_string())
    });

    match result {
        Ok(id_string) => match CString::new(id_string) {
            Ok(c_string) => {
                let ptr = c_string.into_raw();
                IOSSDKResult::success(ptr as *mut std::os::raw::c_void)
            }
            Err(e) => IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InternalError,
                format!("Failed to create C string: {}", e),
            )),
        },
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Put identity to platform and wait for confirmation
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_put_to_platform_and_wait(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    asset_lock_proof_type: u8, // 0 = instant, 1 = chain
    asset_lock_proof_data: *const u8,
    asset_lock_proof_data_len: usize,
    asset_lock_proof_private_key: *const [u8; 32], // Private key as bytes
    signer_handle: *const crate::types::SignerHandle,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || asset_lock_proof_data.is_null()
        || asset_lock_proof_private_key.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const super::signer::IOSSigner);
    let private_key_bytes = *asset_lock_proof_private_key;

    let result: Result<Identity, FFIError> = wrapper.runtime.block_on(async {
        // Convert private key bytes to PrivateKey
        let secp = dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key =
            dpp::dashcore::secp256k1::SecretKey::from_byte_array(&private_key_bytes)
                .map_err(|e| FFIError::InternalError(format!("Invalid private key: {}", e)))?;
        let private_key = PrivateKey::new(secret_key, Network::Dash);

        // Parse asset lock proof data
        let proof_data =
            std::slice::from_raw_parts(asset_lock_proof_data, asset_lock_proof_data_len);

        // For now, create a simple instant asset lock proof as a placeholder
        // In a real implementation, you would parse the proof_data based on asset_lock_proof_type
        let asset_lock_proof = if asset_lock_proof_type == 0 {
            // Instant asset lock proof
            // This is a placeholder - real implementation would deserialize from proof_data
            return Err(FFIError::InternalError(
                "Instant asset lock proof parsing not implemented".to_string(),
            ));
        } else {
            // Chain asset lock proof
            // This is a placeholder - real implementation would deserialize from proof_data
            return Err(FFIError::InternalError(
                "Chain asset lock proof parsing not implemented".to_string(),
            ));
        };

        // Use PutIdentity trait to put identity to platform and wait for response
        let confirmed_identity = identity
            .put_to_platform_and_wait_for_response(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                signer,
                None, // settings (use defaults)
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!(
                    "Failed to put identity to platform and wait: {}",
                    e
                ))
            })?;

        Ok(confirmed_identity)
    });

    match result {
        Ok(confirmed_identity) => {
            let handle = Box::into_raw(Box::new(confirmed_identity)) as *mut IdentityHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
