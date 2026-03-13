//! Shielded transfer (shielded-to-shielded) FFI binding.

use crate::sdk::SDKWrapper;
use crate::shielded::types::{convert_orchard_bundle_params, DashSDKOrchardBundleParams};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::platform::transition::shielded_transfer::TransferShielded;

/// Transfer funds within the shielded pool (shielded-to-shielded).
///
/// Authentication is entirely via Orchard spend authorization signatures
/// embedded in the bundle actions — no identity or platform key required.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `bundle`: Orchard bundle parameters (actions, anchor, proof, binding signature)
/// - `value_balance`: Net value flowing out of the shielded pool (typically 0 for pure transfers, >0 if paying fees)
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle.
/// - `bundle` must be a valid pointer to a `DashSDKOrchardBundleParams`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_transfer(
    sdk_handle: *const SDKHandle,
    bundle: *const DashSDKOrchardBundleParams,
    value_balance: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Bundle params is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .transfer_shielded(orchard_bundle, value_balance, None)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
