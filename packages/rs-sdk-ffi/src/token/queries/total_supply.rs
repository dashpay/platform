use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};
use dash_sdk::dpp::balances::total_single_token_balance::TotalSingleTokenBalance;
use dash_sdk::platform::Fetch;
use std::ffi::{c_char, c_void, CStr, CString};

/// Fetches the total supply of a token
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `token_id` - Base58-encoded token identifier
///
/// # Returns
/// * JSON string with token supply info or null if not found
/// * Error message if operation fails
///
/// # Safety
/// This function is unsafe because it handles raw pointers from C
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_total_supply(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
) -> DashSDKResult {
    match get_token_total_supply(sdk_handle, token_id) {
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

fn get_token_total_supply(
    sdk_handle: *const SDKHandle,
    token_id: *const c_char,
) -> Result<Option<String>, String> {
    // Check for null pointers
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }
    if token_id.is_null() {
        return Err("Token ID is null".to_string());
    }

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create Tokio runtime: {}", e))?;

    let token_id_str = unsafe {
        CStr::from_ptr(token_id)
            .to_str()
            .map_err(|e| format!("Invalid UTF-8 in token ID: {}", e))?
    };
    let wrapper = unsafe { &*(sdk_handle as *const crate::sdk::SDKWrapper) };
    let sdk = wrapper.sdk.clone();

    rt.block_on(async move {
        let token_id_bytes = bs58::decode(token_id_str)
            .into_vec()
            .map_err(|e| format!("Failed to decode token ID: {}", e))?;

        let token_id: [u8; 32] = token_id_bytes
            .try_into()
            .map_err(|_| "Token ID must be exactly 32 bytes".to_string())?;

        let token_id = dash_sdk::platform::Identifier::new(token_id);

        match TotalSingleTokenBalance::fetch(&sdk, token_id).await {
            Ok(Some(balance)) => {
                // Return just the supply number as a string
                Ok(Some(balance.token_supply.to_string()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to fetch token total supply: {}", e)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::create_mock_sdk_handle;

    #[test]
    fn test_get_token_total_supply_null_handle() {
        unsafe {
            let result = dash_sdk_token_get_total_supply(
                std::ptr::null(),
                CString::new("test").unwrap().as_ptr(),
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_get_token_total_supply_null_token_id() {
        let handle = create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_token_get_total_supply(handle, std::ptr::null());
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
