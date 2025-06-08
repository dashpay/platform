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

/// Create a new data contract
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_create(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    schema_json: *const c_char,
    owner_identity_handle: *const rs_sdk_ffi::IdentityHandle,
) -> *mut rs_sdk_ffi::DataContractHandle {
    if sdk_handle.is_null() || schema_json.is_null() || owner_identity_handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_data_contract_create(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            owner_identity_handle,
            schema_json,
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::DataContractHandle
    }
}

/// Put data contract to platform
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_put_to_platform(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    // Note: The FFI function is not exported in rs-sdk-ffi yet
    SwiftDashResult::error(SwiftDashError::not_implemented(
        "Data contract put_to_platform not yet available in FFI",
    ))
}

/// Put data contract to platform and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_put_to_platform_and_wait(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> *mut rs_sdk_ffi::DataContractHandle {
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    // Note: The FFI function is not exported in rs-sdk-ffi yet
    ptr::null_mut()
}

/// Update an existing data contract (Note: updating requires fetching, modifying, and putting back)
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_update(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    contract_id: *const c_char,
    schema_json: *const c_char,
    _version: u32,
) -> SwiftDashResult {
    if sdk_handle.is_null() || contract_id.is_null() || schema_json.is_null() {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    // To update a data contract:
    // 1. Fetch the existing contract
    // 2. Modify its schema
    // 3. Use put_to_platform to broadcast the update
    // This requires proper identity keys and signers which should be handled by the caller
    SwiftDashResult::error(SwiftDashError::not_implemented("Data contract update requires fetching, modifying and putting - use fetch and put_to_platform"))
}

/// Free data contract handle
#[no_mangle]
pub unsafe extern "C" fn swift_dash_data_contract_destroy(
    handle: *mut rs_sdk_ffi::DataContractHandle,
) {
    if !handle.is_null() {
        rs_sdk_ffi::dash_sdk_data_contract_destroy(handle);
    }
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
