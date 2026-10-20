//! `dash_sdk_system_get_current_epoch` — the newest started epoch, via
//! [`ExtendedEpochInfo::fetch_current`].
//!
//! `dash_sdk_system_get_epochs_info` cannot answer "which epoch is it now":
//! an unbounded descending proved query is rejected by the proof verifier,
//! and `start = None, ascending = true` is epoch 0. `fetch_current` runs the
//! two-query probe that resolves the current epoch against proofs, so hosts
//! get a verified answer from one call.

use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use std::ffi::{c_void, CString};

/// Fetch the current (newest started) epoch as a JSON object with the same
/// keys `dash_sdk_system_get_epochs_info` emits per epoch:
/// `{"index","first_block_time","first_block_height","first_core_block_height",
/// "fee_multiplier_permille","protocol_version"}`.
///
/// Returns `NoData` (no error) when Platform has no epoch yet.
///
/// # Safety
/// `sdk_handle` must be a valid SDK handle or null (null ⇒ error result).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_current_epoch(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    match get_current_epoch(sdk_handle) {
        Ok(Some(json)) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::NoData,
                        data: std::ptr::null_mut(),
                        error: Box::into_raw(Box::new(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to create CString: {}", e),
                        ))),
                    }
                }
            };
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: c_str.into_raw() as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::NoData,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

fn get_current_epoch(sdk_handle: *const SDKHandle) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    let rt = crate::runtime::BigStackRuntime::new_isolated()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        match ExtendedEpochInfo::fetch_current(&sdk).await {
            Ok(epoch) => Ok(Some(format!(
                r#"{{"index":{},"first_block_time":{},"first_block_height":{},"first_core_block_height":{},"fee_multiplier_permille":{},"protocol_version":{}}}"#,
                epoch.index(),
                epoch.first_block_time(),
                epoch.first_block_height(),
                epoch.first_core_block_height(),
                epoch.fee_multiplier_permille(),
                epoch.protocol_version()
            ))),
            Err(dash_sdk::Error::EpochNotFound) => Ok(None),
            Err(e) => Err(format!("Failed to fetch current epoch: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_handle_is_an_error() {
        unsafe {
            let result = dash_sdk_system_get_current_epoch(std::ptr::null());
            assert!(!result.error.is_null());
            crate::dash_sdk_error_free(result.error);
        }
    }
}
