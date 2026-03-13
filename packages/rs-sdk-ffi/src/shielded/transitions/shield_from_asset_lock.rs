//! Shield from asset lock (L1 → shielded pool) FFI binding.

use crate::identity::{create_chain_asset_lock_proof, create_instant_asset_lock_proof};
use crate::sdk::SDKWrapper;
use crate::shielded::types::{convert_orchard_bundle_params, DashSDKOrchardBundleParams};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::platform::transition::shield_from_asset_lock::ShieldFromAssetLock;

/// Shield funds from an L1 instant asset lock into the shielded pool.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `instant_lock_bytes`: Serialized instant lock
/// - `instant_lock_len`: Length of instant lock bytes
/// - `transaction_bytes`: Serialized funding transaction
/// - `transaction_len`: Length of transaction bytes
/// - `output_index`: Output index in the transaction
/// - `private_key`: 32-byte ECDSA private key for the asset lock
/// - `bundle`: Orchard bundle parameters
/// - `value_balance`: Net value flowing into the shielded pool
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - All pointers must be valid. `private_key` must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_shield_from_instant_lock(
    sdk_handle: *const SDKHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const u8,
    bundle: *const DashSDKOrchardBundleParams,
    value_balance: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
        || bundle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required pointers are null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let asset_lock_proof = match create_instant_asset_lock_proof(
        instant_lock_bytes,
        instant_lock_len,
        transaction_bytes,
        transaction_len,
        output_index,
    ) {
        Ok(proof) => proof,
        Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
    };

    let pk_bytes = std::slice::from_raw_parts(private_key, 32);

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .shield_from_asset_lock(
                asset_lock_proof,
                pk_bytes,
                orchard_bundle,
                value_balance,
                None,
            )
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Shield funds from an L1 chain asset lock into the shielded pool.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `core_chain_locked_height`: Core chain locked height for the asset lock
/// - `out_point_bytes`: 36-byte outpoint (32 txid + 4 index)
/// - `private_key`: 32-byte ECDSA private key for the asset lock
/// - `bundle`: Orchard bundle parameters
/// - `value_balance`: Net value flowing into the shielded pool
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - All pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_shield_from_chain_lock(
    sdk_handle: *const SDKHandle,
    core_chain_locked_height: u32,
    out_point_bytes: *const [u8; 36],
    private_key: *const u8,
    bundle: *const DashSDKOrchardBundleParams,
    value_balance: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if out_point_bytes.is_null() || private_key.is_null() || bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required pointers are null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let asset_lock_proof =
        match create_chain_asset_lock_proof(core_chain_locked_height, out_point_bytes) {
            Ok(proof) => proof,
            Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
        };

    let pk_bytes = std::slice::from_raw_parts(private_key, 32);

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .shield_from_asset_lock(
                asset_lock_proof,
                pk_bytes,
                orchard_bundle,
                value_balance,
                None,
            )
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
