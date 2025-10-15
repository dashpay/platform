use crate::sdk::SDKWrapper;
use crate::{DashSDKError, DashSDKErrorCode, DataContractHandle, FFIError, SDKHandle};
use dash_sdk::dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dash_sdk::dpp::data_contract::DataContractWithSerialization;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::platform::{Fetch, Identifier};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Result structure for data contract fetch with serialization
#[repr(C)]
pub struct DashSDKDataContractFetchResult {
    /// Handle to the data contract (null on error or if not requested)
    pub contract_handle: *mut DataContractHandle,
    /// JSON representation of the contract (null on error or if not requested)
    pub json_string: *mut c_char,
    /// Serialized contract bytes (null on error or if not requested)
    pub serialized_data: *mut u8,
    /// Length of serialized data
    pub serialized_data_len: usize,
    /// Error information (null on success)
    pub error: *mut DashSDKError,
}

impl DashSDKDataContractFetchResult {
    /// Create a success result with contract data
    pub fn success(
        contract_handle: Option<*mut DataContractHandle>,
        json_string: Option<*mut c_char>,
        serialized_data: Option<Vec<u8>>,
    ) -> Self {
        let (data_ptr, data_len) = if let Some(data) = serialized_data {
            let len = data.len();
            let ptr = Box::into_raw(data.into_boxed_slice()) as *mut u8;
            (ptr, len)
        } else {
            (std::ptr::null_mut(), 0)
        };

        Self {
            contract_handle: contract_handle.unwrap_or(std::ptr::null_mut()),
            json_string: json_string.unwrap_or(std::ptr::null_mut()),
            serialized_data: data_ptr,
            serialized_data_len: data_len,
            error: std::ptr::null_mut(),
        }
    }

    /// Create an error result
    pub fn error(error: DashSDKError) -> Self {
        Self {
            contract_handle: std::ptr::null_mut(),
            json_string: std::ptr::null_mut(),
            serialized_data: std::ptr::null_mut(),
            serialized_data_len: 0,
            error: Box::into_raw(Box::new(error)),
        }
    }
}

/// Fetch a data contract by ID with serialization
///
/// # Safety
/// - `sdk_handle` and `contract_id` must be valid, non-null pointers.
/// - `contract_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - The returned result contains heap-allocated buffers/handles depending on flags; caller must free them using
///   `dash_sdk_data_contract_fetch_result_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_fetch_with_serialization(
    sdk_handle: *const SDKHandle,
    contract_id: *const c_char,
    return_json: bool,
    return_serialized: bool,
) -> DashSDKDataContractFetchResult {
    if sdk_handle.is_null() || contract_id.is_null() {
        return DashSDKDataContractFetchResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or contract ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(contract_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKDataContractFetchResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKDataContractFetchResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract ID: {}", e),
            ))
        }
    };

    let result = wrapper.runtime.block_on(async {
        DataContractWithSerialization::fetch(&wrapper.sdk, id)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(Some((contract, serialization))) => {
            let platform_version = wrapper.sdk.version();

            // Always create a handle since we have the contract
            let handle = Some(Box::into_raw(Box::new(contract.clone())) as *mut DataContractHandle);

            // Prepare JSON if requested
            let json = if return_json {
                match contract.to_json(platform_version) {
                    Ok(json_value) => match serde_json::to_string(&json_value) {
                        Ok(json_string) => match CString::new(json_string) {
                            Ok(c_str) => Some(c_str.into_raw()),
                            Err(e) => {
                                return DashSDKDataContractFetchResult::error(
                                    FFIError::from(e).into(),
                                )
                            }
                        },
                        Err(e) => {
                            return DashSDKDataContractFetchResult::error(FFIError::from(e).into())
                        }
                    },
                    Err(e) => {
                        return DashSDKDataContractFetchResult::error(DashSDKError::new(
                            DashSDKErrorCode::SerializationError,
                            format!("Failed to convert contract to JSON: {}", e),
                        ))
                    }
                }
            } else {
                None
            };

            // Use the serialization if requested, otherwise None
            let serialized = if return_serialized {
                Some(serialization)
            } else {
                None
            };

            DashSDKDataContractFetchResult::success(handle, json, serialized)
        }
        Ok(None) => DashSDKDataContractFetchResult::error(DashSDKError::new(
            DashSDKErrorCode::NotFound,
            "Data contract not found".to_string(),
        )),
        Err(e) => DashSDKDataContractFetchResult::error(e.into()),
    }
}

/// Free the memory allocated for a data contract fetch result
///
/// # Safety
/// - `result` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `result` and all contained pointers become invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_fetch_result_free(
    result: *mut DashSDKDataContractFetchResult,
) {
    if result.is_null() {
        return;
    }

    let result = Box::from_raw(result);

    // Free the contract handle if present
    if !result.contract_handle.is_null() {
        use dash_sdk::platform::DataContract;
        let _ = Box::from_raw(result.contract_handle as *mut DataContract);
    }

    // Free the JSON string if present
    if !result.json_string.is_null() {
        let _ = CString::from_raw(result.json_string);
    }

    // Free the serialized data if present
    if !result.serialized_data.is_null() && result.serialized_data_len > 0 {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(
            result.serialized_data,
            result.serialized_data_len,
        ));
    }

    // Free the error if present
    if !result.error.is_null() {
        let _ = Box::from_raw(result.error);
    }
}
