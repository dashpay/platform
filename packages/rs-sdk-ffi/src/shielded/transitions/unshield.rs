//! Unshield (shielded pool → platform address) FFI binding.

use crate::sdk::SDKWrapper;
use crate::shielded::types::{convert_orchard_bundle_params, DashSDKOrchardBundleParams};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::platform::transition::unshield::UnshieldFunds;

/// Unshield funds from the shielded pool to a platform address.
///
/// Authentication is via Orchard spend authorization signatures in the bundle.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `output_address`: Platform address bytes (typically 21 bytes: 1 type + 20 hash)
/// - `output_address_len`: Length of address bytes
/// - `unshielding_amount`: Amount to unshield (in credits)
/// - `bundle`: Orchard bundle parameters
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - `sdk_handle` must be valid. `output_address` must point to `output_address_len` bytes.
/// - `bundle` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_unshield_funds(
    sdk_handle: *const SDKHandle,
    output_address: *const u8,
    output_address_len: usize,
    unshielding_amount: u64,
    bundle: *const DashSDKOrchardBundleParams,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if output_address.is_null() || output_address_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Output address is null or empty".to_string(),
        ));
    }

    if bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Bundle params is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let address_bytes = std::slice::from_raw_parts(output_address, output_address_len);
    let address = match PlatformAddress::from_bytes(address_bytes) {
        Ok(addr) => addr,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid output address: {}", e),
            ))
        }
    };

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .unshield_funds(address, unshielding_amount, orchard_bundle, None)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
