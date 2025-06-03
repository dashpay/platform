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
        let params = ios_sdk_ffi::IOSSDKDocumentCreateParams {
            data_contract_handle: contract_handle,
            document_type,
            owner_identity_handle: owner_identity_id as *const ios_sdk_ffi::IdentityHandle,
            properties_json: data_json,
        };

        let result = ios_sdk_ffi::ios_sdk_document_create(sdk_handle, &params);

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

        if result.is_null() {
            return ptr::null_mut();
        }

        let ffi_info = &*result;

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

        // Free the original structure
        ios_sdk_ffi::ios_sdk_document_info_free(result);

        Box::into_raw(swift_info)
    }
}

/// Put document to platform and return serialized state transition
#[no_mangle]
pub extern "C" fn swift_dash_document_put_to_platform(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
        let result = ios_sdk_ffi::ios_sdk_document_put_to_platform(
            sdk_handle,
            document_handle,
            data_contract_handle,
            document_type_name,
            entropy,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

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

/// Put document to platform and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_document_put_to_platform_and_wait(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    entropy: *const [u8; 32],
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
        let result = ios_sdk_ffi::ios_sdk_document_put_to_platform_and_wait(
            sdk_handle,
            document_handle,
            data_contract_handle,
            document_type_name,
            entropy,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
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
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    purchaser_id: *const c_char,
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || purchaser_id.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
        let result = ios_sdk_ffi::ios_sdk_document_purchase_to_platform(
            sdk_handle,
            document_handle,
            data_contract_handle,
            document_type_name,
            price,
            purchaser_id,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

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

/// Purchase document from platform and wait for confirmation
#[no_mangle]
pub extern "C" fn swift_dash_document_purchase_to_platform_and_wait(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    purchaser_id: *const c_char,
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || purchaser_id.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
        let result = ios_sdk_ffi::ios_sdk_document_purchase_to_platform_and_wait(
            sdk_handle,
            document_handle,
            data_contract_handle,
            document_type_name,
            price,
            purchaser_id,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

        if !result.error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(result.error);
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
        // This function exists but returns a different type
        let error =
            ios_sdk_ffi::ios_sdk_document_update(ptr::null_mut(), document_handle, properties_json);
        if !error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(error);
            return ptr::null_mut();
        }

        // Since the actual function returns an error pointer, not a handle,
        // we return the original handle if no error occurred
        document_handle
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
        data_contract_handle: contract_handle,
        document_type,
        where_json: if where_clause.is_null() {
            std::ptr::null()
        } else {
            where_clause
        },
        order_by_json: if order_by.is_null() {
            std::ptr::null()
        } else {
            order_by
        },
        limit,
        start_at: if start_after.is_null() {
            0
        } else {
            unsafe {
                CStr::from_ptr(start_after)
                    .to_str()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0)
            }
        },
    };

    unsafe {
        let result = ios_sdk_ffi::ios_sdk_document_search(sdk_handle, &search_params);

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
) -> *mut SwiftDashBinaryData {
    unsafe {
        let error = ios_sdk_ffi::ios_sdk_document_destroy(sdk_handle, document_handle);

        if !error.is_null() {
            ios_sdk_ffi::ios_sdk_error_free(error);
            return ptr::null_mut();
        }

        // The destroy function only returns an error, not binary data
        // Return null for now
        ptr::null_mut()
    }
}

/// Transfer document to another identity
#[no_mangle]
pub extern "C" fn swift_dash_document_transfer_to_identity(
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || recipient_id.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
            sdk_handle,
            document_handle,
            recipient_id,
            data_contract_handle,
            document_type_name,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

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
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    recipient_id: *const c_char,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || recipient_id.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
            sdk_handle,
            document_handle,
            recipient_id,
            data_contract_handle,
            document_type_name,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

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
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut SwiftDashBinaryData {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
        let result = ios_sdk_ffi::ios_sdk_document_update_price_of_document(
            sdk_handle,
            document_handle,
            data_contract_handle,
            document_type_name,
            price,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

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
    sdk_handle: *mut ios_sdk_ffi::SDKHandle,
    document_handle: *mut ios_sdk_ffi::DocumentHandle,
    data_contract_handle: *mut ios_sdk_ffi::DataContractHandle,
    document_type_name: *const c_char,
    price: u64,
    identity_public_key_handle: *mut ios_sdk_ffi::IdentityPublicKeyHandle,
    signer_handle: *mut ios_sdk_ffi::SignerHandle,
    token_payment_info: *const ios_sdk_ffi::IOSSDKTokenPaymentInfo,
    settings: *const SwiftDashPutSettings,
) -> *mut ios_sdk_ffi::DocumentHandle {
    if sdk_handle.is_null()
        || document_handle.is_null()
        || data_contract_handle.is_null()
        || document_type_name.is_null()
        || identity_public_key_handle.is_null()
        || signer_handle.is_null()
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
        let result = ios_sdk_ffi::ios_sdk_document_update_price_of_document_and_wait(
            sdk_handle,
            document_handle,
            data_contract_handle,
            document_type_name,
            price,
            identity_public_key_handle as *const ios_sdk_ffi::IdentityPublicKeyHandle,
            signer_handle as *const ios_sdk_ffi::SignerHandle,
            token_payment_info,
            ffi_settings,
        );

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
