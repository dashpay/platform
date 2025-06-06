use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType, FFIError};
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::query_types::TotalCreditsInPlatform;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

/// Fetches the total credits in the platform
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
///
/// # Returns
/// * JSON string with total credits
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_total_credits_in_platform(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    match get_total_credits_in_platform(sdk_handle) {
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

fn get_total_credits_in_platform(sdk_handle: *const SDKHandle) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        match TotalCreditsInPlatform::fetch_current(&sdk).await {
            Ok(TotalCreditsInPlatform(credits)) => {
                let json = format!(r#"{{"credits":{}}}"#, credits);
                Ok(Some(json))
            }
            Err(e) => Err(format!("Failed to fetch total credits in platform: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_total_credits_in_platform_null_handle() {
        unsafe {
            let result = dash_sdk_system_get_total_credits_in_platform(std::ptr::null());
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_total_credits_in_platform() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_system_get_total_credits_in_platform(handle);
            // Result depends on mock implementation
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
