//! Identity top-up operations

use dash_sdk::dpp::prelude::Identity;
use dash_sdk::platform::Fetch;

use crate::identity::helpers::{
    convert_put_settings, create_instant_asset_lock_proof, parse_private_key,
};
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, DashSDKResultDataType, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Top up an identity with credits using instant lock proof
///
/// # Safety
/// - `sdk_handle`, `identity_handle`, `instant_lock_bytes`, `transaction_bytes`, and `private_key` must be valid, non-null pointers.
/// - Buffer pointers must reference at least the specified lengths.
/// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
/// - On success, returns serialized data; any heap memory inside the result must be freed using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_topup_with_instant_lock(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);

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

        // Use TopUp trait to top up identity
        use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

        let new_balance = identity
            .top_up_identity(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                settings.and_then(|s| s.user_fee_increase),
                settings,
            )
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to top up identity: {}", e)))?;

        // Return the new balance as a string since we don't have the state transition anymore
        Ok(new_balance.to_string().into_bytes())
    });

    match result {
        Ok(serialized_data) => DashSDKResult::success_binary(serialized_data),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Top up an identity with credits using instant lock proof and wait for confirmation
///
/// # Safety
/// - Same requirements as `dash_sdk_identity_topup_with_instant_lock`.
/// - The function may block while waiting for confirmation; input pointers must remain valid throughout.
/// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_topup_with_instant_lock_and_wait(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const [u8; 32],
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);

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

        // Use TopUp trait to top up identity and wait for response
        use dash_sdk::platform::transition::top_up_identity::TopUpIdentity;

        let _new_balance = identity
            .top_up_identity(
                &wrapper.sdk,
                asset_lock_proof,
                &private_key,
                settings.and_then(|s| s.user_fee_increase),
                settings,
            )
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to top up identity: {}", e)))?;

        // Fetch the updated identity after top up
        use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
        let updated_identity = Identity::fetch(&wrapper.sdk, identity.id())
            .await
            .map_err(FFIError::from)?
            .ok_or_else(|| {
                FFIError::InternalError("Failed to fetch updated identity".to_string())
            })?;

        Ok(updated_identity)
    });

    match result {
        Ok(topped_up_identity) => {
            let handle = Box::into_raw(Box::new(topped_up_identity)) as *mut IdentityHandle;
            DashSDKResult::success_handle(
                handle as *mut std::os::raw::c_void,
                DashSDKResultDataType::ResultIdentityHandle,
            )
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
