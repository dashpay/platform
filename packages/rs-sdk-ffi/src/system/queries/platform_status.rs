//! Platform status query

use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::platform::types::epoch::EpochQuery;
use dash_sdk::platform::{FetchMany, LimitQuery};
use std::ffi::CString;
use std::os::raw::c_void;

/// Get platform status including block heights
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_platform_status(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    match get_platform_status(sdk_handle) {
        Ok(json) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult {
                        data_type: DashSDKResultDataType::None,
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
        Err(e) => DashSDKResult {
            data_type: DashSDKResultDataType::None,
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                e,
            ))),
        },
    }
}

fn get_platform_status(sdk_handle: *const SDKHandle) -> Result<String, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    // Get network
    let network_str = match sdk.network {
        dash_sdk::dpp::dashcore::Network::Dash => "mainnet",
        dash_sdk::dpp::dashcore::Network::Testnet => "testnet",
        dash_sdk::dpp::dashcore::Network::Devnet => "devnet",
        dash_sdk::dpp::dashcore::Network::Regtest => "regtest",
        _ => "unknown",
    };

    rt.block_on(async move {
        // Query for the most recent epoch
        let query = LimitQuery {
            query: EpochQuery {
                start: None,
                ascending: false, // Get most recent first
            },
            limit: Some(1),
            start_info: None,
        };

        match ExtendedEpochInfo::fetch_many(&sdk, query).await {
            Ok(epochs) => {
                // Get the first (most recent) epoch
                if let Some((_, Some(epoch))) = epochs.iter().next() {
                    // Calculate current block height
                    // This is an approximation - the actual current block height would need a different query
                    let block_height = epoch.first_block_height();
                    let core_height = epoch.first_core_block_height();

                    let json = format!(
                        r#"{{"version":{},"network":"{}","blockHeight":{},"coreHeight":{}}}"#,
                        10, // Protocol version
                        network_str,
                        block_height,
                        core_height
                    );
                    Ok(json)
                } else {
                    // If no epochs found, return default values
                    let json = format!(
                        r#"{{"version":{},"network":"{}","blockHeight":0,"coreHeight":0}}"#,
                        10, network_str
                    );
                    Ok(json)
                }
            }
            Err(e) => Err(format!("Failed to fetch platform status: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_platform_status_null_handle() {
        unsafe {
            let result = dash_sdk_get_platform_status(std::ptr::null());
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_platform_status() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let _result = dash_sdk_get_platform_status(handle);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
