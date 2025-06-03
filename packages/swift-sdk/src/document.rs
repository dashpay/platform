use crate::identity::SwiftDashBinaryData;
use crate::sdk::SwiftDashPutSettings;
use std::ffi::{CStr, CString};
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

/// Create a new document
#[no_mangle]
pub extern "C" fn swift_dash_document_create(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    owner_identity_id: *const c_char,
    document_type: *const c_char,
    data_json: *const c_char,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || contract_handle.is_null()
        || owner_identity_id.is_null()
        || document_type.is_null()
        || data_json.is_null()
    {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_document_create(
            sdk_handle,
            contract_handle,
            owner_identity_id,
            document_type,
            data_json,
        );

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Fetch a document by ID
#[no_mangle]
pub extern "C" fn swift_dash_document_fetch(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type: *const c_char,
    document_id: *const c_char,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || contract_handle.is_null()
        || document_type.is_null()
        || document_id.is_null()
    {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_document_fetch(
            sdk_handle,
            contract_handle,
            document_type,
            document_id,
        );

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Get document information
#[no_mangle]
pub extern "C" fn swift_dash_document_get_info(
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
) -> *mut SwiftDashDocumentInfo {
    if document_handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_document_get_info(document_handle);

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        if result.data.is_null() {
            return ptr::null_mut();
        }

        let ffi_info_ptr = result.data as *mut ios_sdk_ffi::IOSSDKDocumentInfo;
        let ffi_info = *Box::from_raw(ffi_info_ptr);

        // Convert to Swift-friendly structure
        let swift_info = Box::new(SwiftDashDocumentInfo {
            id: ffi_info.id,                             // Transfer ownership
            owner_id: ffi_info.owner_id,                 // Transfer ownership
            data_contract_id: ffi_info.data_contract_id, // Transfer ownership
            document_type: ffi_info.document_type,       // Transfer ownership
            revision: ffi_info.revision,
            created_at: ffi_info.created_at,
            updated_at: ffi_info.updated_at,
        });

        Box::into_raw(swift_info)
    }
}

/// Put document to platform and return serialized state transition
#[no_mangle]
pub extern "C" fn swift_dash_document_put_to_platform(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || document_handle.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_put_to_platform(
            sdk_handle,
            document_handle,
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

/// Put document to platform and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_document_put_to_platform_and_wait(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null() || document_handle.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_put_to_platform_and_wait(
            sdk_handle,
            document_handle,
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

        if result.data_type != ios_sdk_ffi::IOSSDKResultDataType::DocumentHandle {
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Purchase document from platform and return serialized state transition
#[no_mangle]
pub extern "C" fn swift_dash_document_purchase_to_platform(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || document_handle.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_purchase_to_platform(
            sdk_handle,
            document_handle,
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

/// Purchase document from platform and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_document_purchase_to_platform_and_wait(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null() || document_handle.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_purchase_to_platform_and_wait(
            sdk_handle,
            document_handle,
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

        if result.data_type != ios_sdk_ffi::IOSSDKResultDataType::DocumentHandle {
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Update an existing document
#[no_mangle]
pub extern "C" fn swift_dash_document_update(
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    properties_json: *const c_char,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if document_handle.is_null() || properties_json.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_document_update(document_handle, properties_json);

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
            return ptr::null_mut();
        }

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Search for documents
#[no_mangle]
pub extern "C" fn swift_dash_document_search(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type: *const c_char,
    where_clause: *const c_char,
    order_by: *const c_char,
    limit: u32,
    start_after: *const c_char,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || contract_handle.is_null() || document_type.is_null() {
        return ptr::null_mut();
    }

    // Create search params structure - simplified for Swift
    let search_params = ios_sdk_ffi::IOSSDKDocumentSearchParams {
        contract_handle,
        document_type,
        where_clause: if where_clause.is_null() { std::ptr::null() } else { where_clause },
        order_by: if order_by.is_null() { std::ptr::null() } else { order_by },
        limit,
        start_after: if start_after.is_null() { std::ptr::null() } else { start_after },
    };

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_document_search(sdk_handle, search_params);

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
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

/// Destroy/delete a document
#[no_mangle]
pub extern "C" fn swift_dash_document_destroy(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null() || document_handle.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_destroy(
            sdk_handle,
            document_handle,
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

/// Transfer document to another identity
#[no_mangle]
pub extern "C" fn swift_dash_document_transfer_to_identity(
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if document_handle.is_null() || recipient_id.is_null() || data_contract_handle.is_null()
        || document_type_name.is_null() || signer_handle.is_null()
    {
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
        let result = ios_sdk_ffi::ios_sdk_document_transfer_to_identity(
            document_handle,
            recipient_id,
            data_contract_handle,
            document_type_name,
            public_key_id,
            signer_handle,
            std::ptr::null(), // token_payment_info - simplified for Swift
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

/// Transfer document to another identity and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_document_transfer_to_identity_and_wait(
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if document_handle.is_null() || recipient_id.is_null() || data_contract_handle.is_null()
        || document_type_name.is_null() || signer_handle.is_null()
    {
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
        let result = ios_sdk_ffi::ios_sdk_document_transfer_to_identity_and_wait(
            document_handle,
            recipient_id,
            data_contract_handle,
            document_type_name,
            public_key_id,
            signer_handle,
            std::ptr::null(), // token_payment_info - simplified for Swift
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

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Update the price of a document
#[no_mangle]
pub extern "C" fn swift_dash_document_update_price(
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if document_handle.is_null() || data_contract_handle.is_null() || document_type_name.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_update_price_of_document(
            document_handle,
            data_contract_handle,
            document_type_name,
            price,
            public_key_id,
            signer_handle,
            std::ptr::null(), // token_payment_info - simplified for Swift
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

/// Update the price of a document and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_document_update_price_and_wait(
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    public_key_id: u32,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if document_handle.is_null() || data_contract_handle.is_null() || document_type_name.is_null() || signer_handle.is_null() {
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
        let result = ios_sdk_ffi::ios_sdk_document_update_price_of_document_and_wait(
            document_handle,
            data_contract_handle,
            document_type_name,
            price,
            public_key_id,
            signer_handle,
            std::ptr::null(), // token_payment_info - simplified for Swift
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

        result.data as *mut ios_sdk_ffi::DocumentHandle
    }
}

/// Free a Swift document info structure
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
