use crate::sdk::SwiftDashPutSettings;
use crate::identity::SwiftDashBinaryData;
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Fetch a data contract by ID
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_fetch(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    contract_id: *const c_char,
) -> *mut ios_sdk_ffi::DataContractHandle {
    if sdk_handle.is_null() || contract_id.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_data_contract_fetch(sdk_handle, contract_id);
        
        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DataContractHandle
    }
}

/// Create a new data contract from JSON schema
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_create(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    owner_identity_id: *const c_char,
    schema_json: *const c_char,
) -> *mut ios_sdk_ffi::DataContractHandle {
    if sdk_handle.is_null() || owner_identity_id.is_null() || schema_json.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_data_contract_create(
            sdk_handle,
            owner_identity_id,
            schema_json,
        );
        
        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DataContractHandle
    }
}

/// Get data contract information as JSON string
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_get_info(
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
) -> *mut c_char {
    if contract_handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_data_contract_get_info(contract_handle);
        
        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != ios_sdk_ffi::IOSSDKResultDataType::String {
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Get schema for a specific document type
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_get_schema(
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type: *const c_char,
) -> *mut c_char {
    if contract_handle.is_null() || document_type.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_data_contract_get_schema(contract_handle, document_type);
        
        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != ios_sdk_ffi::IOSSDKResultDataType::String {
            return ptr::null_mut();
        }

        result.data as *mut c_char
    }
}

/// Put data contract to platform and return serialized state transition
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_put_to_platform(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || contract_handle.is_null() || signer_handle.is_null() {
        return ptr::null_mut();
    }

    let ffi_settings: *const ios_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_data_contract_put_to_platform(
            sdk_handle,
            contract_handle,
            public_key_id,
            signer_handle,
            ffi_settings,
        );
        
        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }
        
        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != ios_sdk_ffi::IOSSDKResultDataType::BinaryData {
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_binary_ptr = result.data as *mut ios_sdk_ffi::IOSSDKBinaryData;
        let ffi_binary = *Box::from_raw(ffi_binary_ptr);
        
        // Convert to Swift-friendly structure
        let swift_binary = Box::new(SwiftDashBinaryData {
            data: ffi_binary.data, // Transfer ownership
            len: ffi_binary.len,
        });

        Box::into_raw(swift_binary)
    }
}

/// Put data contract to platform and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_data_contract_put_to_platform_and_wait(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DataContractHandle {
    if sdk_handle.is_null() || contract_handle.is_null() || signer_handle.is_null() {
        return ptr::null_mut();
    }

    let ffi_settings: *const ios_sdk_ffi::IOSSDKPutSettings = if settings.is_null() {
        ptr::null()
    } else {
        unsafe {
            let swift_settings = *settings;
            let ffi_settings = Box::new(swift_settings.into());
            Box::into_raw(ffi_settings)
        }
    };

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_data_contract_put_to_platform_and_wait(
            sdk_handle,
            contract_handle,
            public_key_id,
            signer_handle,
            ffi_settings,
        );
        
        // Clean up settings if we allocated them
        if !ffi_settings.is_null() {
            let _ = Box::from_raw(ffi_settings);
        }
        
        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data_type != ios_sdk_ffi::IOSSDKResultDataType::DataContractHandle {
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DataContractHandle
    }
}