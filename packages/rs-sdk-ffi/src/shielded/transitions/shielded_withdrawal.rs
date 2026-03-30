//! Shielded withdrawal (shielded pool → L1) FFI binding.

use crate::sdk::SDKWrapper;
use crate::shielded::types::{convert_orchard_bundle_params, DashSDKOrchardBundleParams};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::identity::core_script::CoreScript;
use dash_sdk::dpp::withdrawal::Pooling;
use dash_sdk::platform::transition::shielded_withdrawal::WithdrawShielded;

/// Withdraw funds from the shielded pool to a Core (L1) address.
///
/// Authentication is via Orchard spend authorization signatures in the bundle.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `unshielding_amount`: Amount to withdraw (in credits)
/// - `bundle`: Orchard bundle parameters
/// - `core_fee_per_byte`: Core fee per byte for the withdrawal transaction
/// - `pooling`: Pooling mode (0 = Never, 1 = IfAvailable, 2 = Standard)
/// - `output_script`: Raw Core output script bytes
/// - `output_script_len`: Length of output script bytes
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - All pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_withdraw(
    sdk_handle: *const SDKHandle,
    unshielding_amount: u64,
    bundle: *const DashSDKOrchardBundleParams,
    core_fee_per_byte: u32,
    pooling: u8,
    output_script: *const u8,
    output_script_len: usize,
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

    if output_script.is_null() || output_script_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Output script is null or empty".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let pooling_enum = match pooling {
        0 => Pooling::Never,
        1 => Pooling::IfAvailable,
        2 => Pooling::Standard,
        _ => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid pooling value: {} (expected 0, 1, or 2)", pooling),
            ))
        }
    };

    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::new(
        dash_sdk::dpp::dashcore::blockdata::script::ScriptBuf::from_bytes(script_bytes.to_vec()),
    );

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .withdraw_shielded(
                unshielding_amount,
                orchard_bundle,
                core_fee_per_byte,
                pooling_enum,
                core_script,
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
