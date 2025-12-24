use crate::error::{DashSDKError, DashSDKErrorCode, FFIError};
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKResult, SDKHandle};
use dash_sdk::dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Fetch a data contract by ID and return as JSON
///
/// # Safety
/// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
/// - `contract_id` must point to a NUL-terminated C string that remains valid for the duration of the call.
/// - On success, returns a heap-allocated C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_fetch_json(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || contract_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or contract ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract ID: {}", e),
            ))
        }
    };

    let result = wrapper.runtime.block_on(async {
        DataContract::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some(contract)) => {
            // Get the platform version
            let platform_version = wrapper.sdk.version();

            // Convert to JSON
            match contract.to_json(platform_version) {
                Ok(json_value) => match serde_json::to_string(&json_value) {
                    Ok(json_string) => match CString::new(json_string) {
                        Ok(c_str) => {
                            DashSDKResult::success(c_str.into_raw() as *mut std::os::raw::c_void)
                        }
                        Err(e) => DashSDKResult::error(FFIError::from(e).into()),
                    },
                    Err(e) => DashSDKResult::error(FFIError::from(e).into()),
                },
                Err(e) => DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("Failed to convert contract to JSON: {}", e),
                )),
            }
        }
        Ok(None) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::NotFound,
            "Data contract not found".to_string(),
        )),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
