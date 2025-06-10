//! Identity put-to-platform operations

use dash_sdk::dpp::prelude::Identity;
use dash_sdk::platform::transition::put_identity::PutIdentity;

use crate::identity::helpers::{
    convert_put_settings, create_chain_asset_lock_proof, create_instant_asset_lock_proof,
    parse_private_key,
};
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, DashSDKResultDataType, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Put identity to platform with instant lock proof
///
/// # Parameters
/// - `instant_lock_bytes`: Serialized InstantLock data
/// - `transaction_bytes`: Serialized Transaction data
/// - `output_index`: Index of the output in the transaction payload
/// - `private_key`: 32-byte private key associated with the asset lock
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_put_to_platform_with_instant_lock(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Create instant asset lock proof
        let asset_lock_proof = create_instant_asset_lock_proof(
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
        )?;

        // Parse private key
        let private_key = parse_private_key(private_key)?;

        // Convert settings
        let settings = convert_put_settings(put_settings);

        // Use PutIdentity trait to put identity to platform
        let state_transition = identity
            .put_to_platform(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put identity to platform: {}", e))
            })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Put identity to platform with instant lock proof and wait for confirmation
///
/// # Parameters
/// - `instant_lock_bytes`: Serialized InstantLock data
/// - `transaction_bytes`: Serialized Transaction data
/// - `output_index`: Index of the output in the transaction payload
/// - `private_key`: 32-byte private key associated with the asset lock
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Handle to the confirmed identity on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_put_to_platform_with_instant_lock_and_wait(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);

    let result: Result<Identity, FFIError> = wrapper.runtime.block_on(async {
        // Create instant asset lock proof
        let asset_lock_proof = create_instant_asset_lock_proof(
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
        )?;

        // Parse private key
        let private_key = parse_private_key(private_key)?;

        // Convert settings
        let settings = convert_put_settings(put_settings);

        // Use PutIdentity trait to put identity to platform and wait for response
        let confirmed_identity = identity
            .put_to_platform_and_wait_for_response(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                signer,
                settings,
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
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Put identity to platform with chain lock proof
///
/// # Parameters
/// - `core_chain_locked_height`: Core height at which the transaction was chain locked
/// - `out_point`: 36-byte OutPoint (32-byte txid + 4-byte vout)
/// - `private_key`: 32-byte private key associated with the asset lock
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_put_to_platform_with_chain_lock(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    core_chain_locked_height: u32,
    out_point: *const [u8; 36],
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || out_point.is_null()
        || private_key.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);

    let result: Result<Vec<u8>, FFIError> = wrapper.runtime.block_on(async {
        // Create chain asset lock proof
        let asset_lock_proof = create_chain_asset_lock_proof(core_chain_locked_height, out_point)?;

        // Parse private key
        let private_key = parse_private_key(private_key)?;

        // Convert settings
        let settings = convert_put_settings(put_settings);

        // Use PutIdentity trait to put identity to platform
        let state_transition = identity
            .put_to_platform(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to put identity to platform: {}", e))
            })?;

        // Serialize the state transition with bincode
        let config = bincode::config::standard();
        bincode::encode_to_vec(&state_transition, config).map_err(|e| {
            FFIError::InternalError(format!("Failed to serialize state transition: {}", e))
        })
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Put identity to platform with chain lock proof and wait for confirmation
///
/// # Parameters
/// - `core_chain_locked_height`: Core height at which the transaction was chain locked
/// - `out_point`: 36-byte OutPoint (32-byte txid + 4-byte vout)
/// - `private_key`: 32-byte private key associated with the asset lock
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// Handle to the confirmed identity on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_put_to_platform_with_chain_lock_and_wait(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    core_chain_locked_height: u32,
    out_point: *const [u8; 36],
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || out_point.is_null()
        || private_key.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const crate::signer::IOSSigner);

    let result: Result<Identity, FFIError> = wrapper.runtime.block_on(async {
        // Create chain asset lock proof
        let asset_lock_proof = create_chain_asset_lock_proof(core_chain_locked_height, out_point)?;

        // Parse private key
        let private_key = parse_private_key(private_key)?;

        // Convert settings
        let settings = convert_put_settings(put_settings);

        // Use PutIdentity trait to put identity to platform and wait for response
        let confirmed_identity = identity
            .put_to_platform_and_wait_for_response(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                signer,
                settings,
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
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
