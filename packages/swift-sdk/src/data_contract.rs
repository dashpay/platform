use crate::error::{SwiftDashError, SwiftDashResult};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Information about a data contract
#[repr(C)]
pub struct SwiftDashDataContractInfo {
    pub id: *mut c_char,
    pub owner_id: *mut c_char,
    pub version: u32,
    pub schema_json: *mut c_char,
}

/// Fetch a data contract by ID
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_fetch(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    contract_id: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null() || contract_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_data_contract_fetch(sdk_handle, contract_id);

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Get data contract history
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_get_history(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    contract_id: *const c_char,
    limit: u32,
    offset: u32,
) -> *mut c_char {
    if sdk_handle.is_null() || contract_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_data_contract_fetch_history(
            sdk_handle,
            contract_id,
            limit,
            offset,
            0, // start_at_ms parameter
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Create a new data contract (simplified - returns not implemented)
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_create(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    schema_json: *const c_char,
    owner_id: *const c_char,
) -> SwiftDashResult {
    if sdk_handle.is_null() || schema_json.is_null() || owner_id.is_null() {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter("Missing required parameters"));
    }

    // Data contract creation requires complex state transition setup
    SwiftDashResult::error(SwiftDashError::not_implemented("Data contract creation not yet implemented"))
}

/// Update an existing data contract (simplified - returns not implemented)
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_update(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    contract_id: *const c_char,
    schema_json: *const c_char,
    _version: u32,
) -> SwiftDashResult {
    if sdk_handle.is_null() || contract_id.is_null() || schema_json.is_null() {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter("Missing required parameters"));
    }

    // Data contract updates require complex state transition setup
    SwiftDashResult::error(SwiftDashError::not_implemented("Data contract update not yet implemented"))
}

/// Free data contract info structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_data_contract_info_free(info: *mut SwiftDashDataContractInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    if !info.id.is_null() {
        let _ = CString::from_raw(info.id);
    }
    if !info.owner_id.is_null() {
        let _ = CString::from_raw(info.owner_id);
    }
    if !info.schema_json.is_null() {
        let _ = CString::from_raw(info.schema_json);
    }
}