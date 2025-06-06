use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType, FFIError};
use dash_sdk::platform::Fetch;
use dash_sdk::query_types::PrefundedSpecializedBalance;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches a prefunded specialized balance
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `id` - Base58-encoded identifier
///
/// # Returns
/// * JSON string with balance or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_prefunded_specialized_balance(
    sdk_handle: *const SDKHandle,
    id: *const c_char,
) -> DashSDKResult {
    match get_prefunded_specialized_balance(sdk_handle, id) {
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

fn get_prefunded_specialized_balance(
    sdk_handle: *const SDKHandle,
    id: *const c_char,
) -> Result<Option<String>, String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let id_str = unsafe {
        CStr::from_ptr(id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in ID: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let id_bytes = bs58::decode(id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode ID: {}", e))?;

        let id: [u8; 32] = id_bytes
            .try_into()
            .map_err(|_| "ID must be exactly 32 bytes".to_string())?;

        let id = dash_sdk::platform::Identifier::new(id);

        match PrefundedSpecializedBalance::fetch(&sdk, id).await {
            Ok(Some(balance)) => {
                let json = format!(r#"{{"balance":{}}}"#, balance.to_credits());
                Ok(Some(json))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!(
                "Failed to fetch prefunded specialized balance: {}",
                e
            )),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_prefunded_specialized_balance_null_handle() {
        unsafe {
            let result = dash_sdk_system_get_prefunded_specialized_balance(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_prefunded_specialized_balance_null_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result =
                dash_sdk_system_get_prefunded_specialized_balance(handle, std::ptr::null());
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
