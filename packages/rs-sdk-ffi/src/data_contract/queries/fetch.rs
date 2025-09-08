use crate::sdk::SDKWrapper;
use crate::{
    DashSDKError, DashSDKErrorCode, DashSDKResult, DataContractHandle, FFIError, SDKHandle,
};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Fetch a data contract by ID
///
/// # Safety
/// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
/// - `contract_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a heap-allocated handle which must be destroyed with the SDK's destroy function.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_fetch(
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
            let handle = Box::into_raw(Box::new(contract)) as *mut DataContractHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Ok(None) => {
            // Mirror rs-sdk semantics: return success with no data when not found
            DashSDKResult::success(std::ptr::null_mut())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
