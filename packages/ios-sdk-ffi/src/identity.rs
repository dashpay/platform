//! Identity operations

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::time::Duration;

use dash_sdk::platform::transition::put_identity::PutIdentity;
use dash_sdk::platform::transition::put_settings::PutSettings;
use dash_sdk::platform::Fetch;
use dash_sdk::RequestSettings;
use dpp::dashcore::{Network, PrivateKey};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::{AssetLockProof, Identifier, Identity, UserFeeIncrease};
use dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dpp::state_transition::StateTransitionSigningOptions;
use platform_value::string_encoding::Encoding;

use crate::sdk::SDKWrapper;
use crate::types::{
    IOSSDKIdentityInfo, IOSSDKPutSettings, IOSSDKResultDataType, IdentityHandle, SDKHandle,
};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Helper function to convert IOSSDKPutSettings to PutSettings
unsafe fn convert_put_settings(put_settings: *const IOSSDKPutSettings) -> Option<PutSettings> {
    if put_settings.is_null() {
        None
    } else {
        let ios_settings = &*put_settings;

        // Convert request settings
        let mut request_settings = RequestSettings::default();
        if ios_settings.connect_timeout_ms > 0 {
            request_settings.connect_timeout =
                Some(Duration::from_millis(ios_settings.connect_timeout_ms));
        }
        if ios_settings.timeout_ms > 0 {
            request_settings.timeout = Some(Duration::from_millis(ios_settings.timeout_ms));
        }
        if ios_settings.retries > 0 {
            request_settings.retries = Some(ios_settings.retries as usize);
        }
        request_settings.ban_failed_address = Some(ios_settings.ban_failed_address);

        // Convert other settings
        let identity_nonce_stale_time_s = if ios_settings.identity_nonce_stale_time_s > 0 {
            Some(ios_settings.identity_nonce_stale_time_s)
        } else {
            None
        };

        let user_fee_increase = if ios_settings.user_fee_increase > 0 {
            Some(ios_settings.user_fee_increase as UserFeeIncrease)
        } else {
            None
        };

        let signing_options = StateTransitionSigningOptions {
            allow_signing_with_any_security_level: ios_settings
                .allow_signing_with_any_security_level,
            allow_signing_with_any_purpose: ios_settings.allow_signing_with_any_purpose,
        };

        let state_transition_creation_options = Some(StateTransitionCreationOptions {
            signing_options,
            batch_feature_version: None,
            method_feature_version: None,
            base_feature_version: None,
        });

        let wait_timeout = if ios_settings.wait_timeout_ms > 0 {
            Some(Duration::from_millis(ios_settings.wait_timeout_ms))
        } else {
            None
        };

        Some(PutSettings {
            request_settings,
            identity_nonce_stale_time_s,
            user_fee_increase,
            state_transition_creation_options,
            wait_timeout,
        })
    }
}

/// Helper function to parse private key
unsafe fn parse_private_key(private_key_bytes: *const [u8; 32]) -> Result<PrivateKey, FFIError> {
    let key_bytes = *private_key_bytes;
    let secret_key = dpp::dashcore::secp256k1::SecretKey::from_byte_array(&key_bytes)
        .map_err(|e| FFIError::InternalError(format!("Invalid private key: {}", e)))?;
    Ok(PrivateKey::new(secret_key, Network::Dash))
}

/// Helper function to create instant asset lock proof from components
unsafe fn create_instant_asset_lock_proof(
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
) -> Result<AssetLockProof, FFIError> {
    use dpp::dashcore::consensus::deserialize;
    use dpp::identity::state_transition::asset_lock_proof::instant::InstantAssetLockProof;

    // Deserialize instant lock
    let instant_lock_data = std::slice::from_raw_parts(instant_lock_bytes, instant_lock_len);
    let instant_lock = deserialize(instant_lock_data).map_err(|e| {
        FFIError::InternalError(format!("Failed to deserialize instant lock: {}", e))
    })?;

    // Deserialize transaction
    let transaction_data = std::slice::from_raw_parts(transaction_bytes, transaction_len);
    let transaction = deserialize(transaction_data).map_err(|e| {
        FFIError::InternalError(format!("Failed to deserialize transaction: {}", e))
    })?;

    // Create instant asset lock proof
    let instant_proof = InstantAssetLockProof::new(instant_lock, transaction, output_index);

    Ok(AssetLockProof::Instant(instant_proof))
}

/// Helper function to create chain asset lock proof from components
unsafe fn create_chain_asset_lock_proof(
    core_chain_locked_height: u32,
    out_point_bytes: *const [u8; 36],
) -> Result<AssetLockProof, FFIError> {
    use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;

    let out_point = *out_point_bytes;

    // Create chain asset lock proof
    let chain_proof = ChainAssetLockProof::new(core_chain_locked_height, out_point);

    Ok(AssetLockProof::Chain(chain_proof))
}

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
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::IdentityHandle,
            )
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

/// Put identity to platform with instant lock proof
///
/// # Parameters
/// - `instant_lock_bytes`: Serialized InstantLock data
/// - `transaction_bytes`: Serialized Transaction data
/// - `output_index`: Index of the output in the transaction payload
/// - `private_key`: 32-byte private key associated with the asset lock
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_put_to_platform_with_instant_lock(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const crate::types::IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
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
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
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
pub unsafe extern "C" fn ios_sdk_identity_put_to_platform_with_instant_lock_and_wait(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const crate::types::IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
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
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::IdentityHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
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
pub unsafe extern "C" fn ios_sdk_identity_put_to_platform_with_chain_lock(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    core_chain_locked_height: u32,
    out_point: *const [u8; 36],
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const crate::types::IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || out_point.is_null()
        || private_key.is_null()
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
        Ok(serialized_data) => IOSSDKResult::success_binary(serialized_data),
        Err(e) => IOSSDKResult::error(e.into()),
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
pub unsafe extern "C" fn ios_sdk_identity_put_to_platform_with_chain_lock_and_wait(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    core_chain_locked_height: u32,
    out_point: *const [u8; 36],
    private_key: *const [u8; 32],
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const crate::types::IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || out_point.is_null()
        || private_key.is_null()
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
            IOSSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                IOSSDKResultDataType::IdentityHandle,
            )
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
