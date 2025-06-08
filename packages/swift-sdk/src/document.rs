use crate::error::{SwiftDashError, SwiftDashResult};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// Information about a document
#[repr(C)]
pub struct SwiftDashDocumentInfo {
    pub id: *mut c_char,
    pub owner_id: *mut c_char,
    pub data_contract_id: *mut c_char,
    pub document_type: *mut c_char,
    pub revision: u64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Fetch a document by ID (simplified - returns not implemented)
#[no_mangle]
pub extern "C" fn swift_dash_document_fetch(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    data_contract_id: *const c_char,
    document_type: *const c_char,
    document_id: *const c_char,
) -> *mut c_char {
    if sdk_handle.is_null()
        || data_contract_id.is_null()
        || document_type.is_null()
        || document_id.is_null()
    {
        return ptr::null_mut();
    }

    // Document fetching requires proper data contract handle setup
    ptr::null_mut()
}

/// Search for documents (simplified - returns not implemented)
#[no_mangle]
pub extern "C" fn swift_dash_document_search(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    data_contract_id: *const c_char,
    document_type: *const c_char,
    _query_json: *const c_char,
    _limit: u32,
) -> *mut c_char {
    if sdk_handle.is_null() || data_contract_id.is_null() || document_type.is_null() {
        return ptr::null_mut();
    }

    // Document search requires proper search parameters setup
    ptr::null_mut()
}

/// Document creation parameters
#[repr(C)]
pub struct SwiftDashDocumentCreateParams {
    pub data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    pub document_type: *const c_char,
    pub owner_identity_handle: *const rs_sdk_ffi::IdentityHandle,
    pub properties_json: *const c_char,
}

/// Create a new document
#[no_mangle]
pub extern "C" fn swift_dash_document_create(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    params: *const SwiftDashDocumentCreateParams,
) -> *mut rs_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null() || params.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let params = &*params;
        if params.data_contract_handle.is_null()
            || params.document_type.is_null()
            || params.owner_identity_handle.is_null()
            || params.properties_json.is_null()
        {
            return ptr::null_mut();
        }

        let ffi_params = rs_sdk_ffi::DashSDKDocumentCreateParams {
            data_contract_handle: params.data_contract_handle,
            document_type: params.document_type,
            owner_identity_handle: params.owner_identity_handle,
            properties_json: params.properties_json,
        };

        let result = rs_sdk_ffi::dash_sdk_document_create(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            &ffi_params,
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::DocumentHandle
    }
}

/// Put document to platform
#[no_mangle]
pub extern "C" fn swift_dash_document_put_to_platform(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    document_handle: *const rs_sdk_ffi::DocumentHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || entropy.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_document_put_to_platform(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            document_handle,
            data_contract_handle,
            document_type_name,
            entropy,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // token_payment_info
            ptr::null(), // put_settings
            ptr::null(), // state_transition_creation_options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        // Extract binary data from result
        if result.data_type == rs_sdk_ffi::DashSDKResultDataType::BinaryData
            && !result.data.is_null()
        {
            let binary_data = result.data as *const rs_sdk_ffi::DashSDKBinaryData;
            let binary = &*binary_data;
            SwiftDashResult::success_binary(binary.data as *mut std::os::raw::c_void, binary.len)
        } else {
            SwiftDashResult::success()
        }
    }
}

/// Put document to platform and wait
#[no_mangle]
pub extern "C" fn swift_dash_document_put_to_platform_and_wait(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    document_handle: *const rs_sdk_ffi::DocumentHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> *mut rs_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || entropy.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_document_put_to_platform_and_wait(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            document_handle,
            data_contract_handle,
            document_type_name,
            entropy,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // token_payment_info
            ptr::null(), // put_settings
            ptr::null(), // state_transition_creation_options
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::DocumentHandle
    }
}

/// Replace document on platform
#[no_mangle]
pub extern "C" fn swift_dash_document_replace_on_platform(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    document_handle: *const rs_sdk_ffi::DocumentHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_document_replace_on_platform(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            document_handle,
            data_contract_handle,
            document_type_name,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // token_payment_info
            ptr::null(), // put_settings
            ptr::null(), // state_transition_creation_options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        // Extract binary data from result
        if result.data_type == rs_sdk_ffi::DashSDKResultDataType::BinaryData
            && !result.data.is_null()
        {
            let binary_data = result.data as *const rs_sdk_ffi::DashSDKBinaryData;
            let binary = &*binary_data;
            SwiftDashResult::success_binary(binary.data as *mut std::os::raw::c_void, binary.len)
        } else {
            SwiftDashResult::success()
        }
    }
}

/// Replace document on platform and wait
#[no_mangle]
pub extern "C" fn swift_dash_document_replace_on_platform_and_wait(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    document_handle: *const rs_sdk_ffi::DocumentHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> *mut rs_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return ptr::null_mut();
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_document_replace_on_platform_and_wait(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            document_handle,
            data_contract_handle,
            document_type_name,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // token_payment_info
            ptr::null(), // put_settings
            ptr::null(), // state_transition_creation_options
        );

        if !result.error.is_null() {
            let _ = Box::from_raw(result.error);
            return ptr::null_mut();
        }

        result.data as *mut rs_sdk_ffi::DocumentHandle
    }
}

/// Delete a document
#[no_mangle]
pub extern "C" fn swift_dash_document_delete(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    document_handle: *const rs_sdk_ffi::DocumentHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_document_delete(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            document_handle,
            data_contract_handle,
            document_type_name,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // token_payment_info
            ptr::null(), // put_settings
            ptr::null(), // state_transition_creation_options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        // Extract binary data from result
        if result.data_type == rs_sdk_ffi::DashSDKResultDataType::BinaryData
            && !result.data.is_null()
        {
            let binary_data = result.data as *const rs_sdk_ffi::DashSDKBinaryData;
            let binary = &*binary_data;
            SwiftDashResult::success_binary(binary.data as *mut std::os::raw::c_void, binary.len)
        } else {
            SwiftDashResult::success()
        }
    }
}

/// Delete a document and wait
#[no_mangle]
pub extern "C" fn swift_dash_document_delete_and_wait(
    sdk_handle: *const rs_sdk_ffi::SDKHandle,
    document_handle: *const rs_sdk_ffi::DocumentHandle,
    data_contract_handle: *const rs_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *const rs_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *const rs_sdk_ffi::SignerHandle,
) -> SwiftDashResult {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
    {
        return SwiftDashResult::error(SwiftDashError::invalid_parameter(
            "Missing required parameters",
        ));
    }

    unsafe {
        let result = rs_sdk_ffi::dash_sdk_document_delete_and_wait(
            sdk_handle as *mut rs_sdk_ffi::SDKHandle,
            document_handle,
            data_contract_handle,
            document_type_name,
            identity_public_key_handle,
            signer_handle,
            ptr::null(), // token_payment_info
            ptr::null(), // put_settings
            ptr::null(), // state_transition_creation_options
        );

        if !result.error.is_null() {
            let error = Box::from_raw(result.error);
            return SwiftDashResult::error(SwiftDashError::from_ffi_error(&*error));
        }

        SwiftDashResult::success()
    }
}

/// Free document handle
#[no_mangle]
pub unsafe extern "C" fn swift_dash_document_destroy(
    sdk_handle: *mut rs_sdk_ffi::SDKHandle,
    handle: *mut rs_sdk_ffi::DocumentHandle,
) {
    if !sdk_handle.is_null() && !handle.is_null() {
        rs_sdk_ffi::dash_sdk_document_destroy(sdk_handle, handle);
    }
}

/// Free document info structure
#[no_mangle]
pub unsafe extern "C" fn swift_dash_document_info_free(info: *mut SwiftDashDocumentInfo) {
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
    if !info.data_contract_id.is_null() {
        let _ = CString::from_raw(info.data_contract_id);
    }
    if !info.document_type.is_null() {
        let _ = CString::from_raw(info.document_type);
    }
}
