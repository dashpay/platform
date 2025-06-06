use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::block::extended_epoch_info::v0::ExtendedEpochInfoV0Getters;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::platform::types::epoch::EpochQuery;
use dash_sdk::platform::{FetchMany, LimitQuery};
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches information about multiple epochs
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `start_epoch` - Starting epoch index (optional, null for default)
/// * `count` - Number of epochs to retrieve
/// * `ascending` - Whether to return epochs in ascending order
///
/// # Returns
/// * JSON array of epoch information or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_epochs_info(
    sdk_handle: *const SDKHandle,
    start_epoch: *const c_char,
    count: u32,
    ascending: bool,
) -> DashSDKResult {
    match get_epochs_info(sdk_handle, start_epoch, count, ascending) {
        Ok(Some(json)) => {
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
        Ok(None) => DashSDKResult {
            data_type: DashSDKResultDataType::None,
            data: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
        },
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

fn get_epochs_info(
    sdk_handle: *const SDKHandle,
    start_epoch: *const c_char,
    count: u32,
    ascending: bool,
) -> Result<Option<String>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let start = if start_epoch.is_null() {
            None
        } else {
            let start_str = unsafe {
                CStr::from_ptr(start_epoch)
                    .to_str()
                    .map_err(|e| format!("Invalid UTF-8 in start epoch: {}", e))?
            };
            Some(
                start_str
                    .parse::<u16>()
                    .map_err(|e| format!("Failed to parse start epoch: {}", e))?,
            )
        };

        let query = LimitQuery {
            query: EpochQuery { start, ascending },
            limit: Some(count),
            start_info: None,
        };

        match ExtendedEpochInfo::fetch_many(&sdk, query).await {
            Ok(epochs) => {
                if epochs.is_empty() {
                    return Ok(None);
                }

                let epochs_json: Vec<String> = epochs
                    .values()
                    .filter_map(|epoch_opt| {
                        epoch_opt.as_ref().map(|epoch| {
                            format!(
                                r#"{{"index":{},"first_block_time":{},"first_block_height":{},"first_core_block_height":{},"fee_multiplier_permille":{},"protocol_version":{}}}"#,
                                epoch.index(),
                                epoch.first_block_time(),
                                epoch.first_block_height(),
                                epoch.first_core_block_height(),
                                epoch.fee_multiplier_permille(),
                                epoch.protocol_version()
                            )
                        })
                    })
                    .collect();

                Ok(Some(format!("[{}]", epochs_json.join(","))))
            }
            Err(e) => Err(format!("Failed to fetch epochs info: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_epochs_info_null_handle() {
        unsafe {
            let result =
                dash_sdk_system_get_epochs_info(std::ptr::null(), std::ptr::null(), 10, true);
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_epochs_info_with_start() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let _result = dash_sdk_system_get_epochs_info(
                handle,
                CString::new("100").unwrap().as_ptr(),
                10,
                true,
            );
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
